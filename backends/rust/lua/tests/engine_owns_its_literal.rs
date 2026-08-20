// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The engine that spells a literal is the engine that has to read it back.
//
// §scxml-6.4.1 has the parent evaluate an `<invoke>`'s params in its own data
// model and hand the values to the child, and the child seeds them by
// evaluating source. So the literal is not decoration: it is fed straight to
// an interpreter, and the only interpreter that can be relied on to accept it
// is the one that wrote it. That is why `to_script_literal` is a method on
// `IScriptEngine` and not on `ScriptValue`, which knew Lua's grammar while
// belonging to no engine at all.
//
// The round trip below is what a table of expected strings cannot claim: it
// does not assert that the text LOOKS like Lua, it asserts that this engine
// parses it back into the value it started from. A JavaScript spelling would
// still look plausible and would fail here — `[1,2]` is a syntax error in
// Lua, and `null` is a nil global, which is the quieter of the two failures
// and the reason the value has to come back rather than merely load.
//
// Sibling: `backends/rust/runtime/tests/wire_value_is_not_engine_source.rs`
// holds the second grammar and the wire table.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue};

fn round_trip(engine: &LuaEngine, session: &str, value: &ScriptValue) -> ScriptValue {
    let literal = engine.to_script_literal(value);
    engine
        .evaluate_expression(session, &literal)
        .unwrap_or_else(|e| panic!("this engine could not read its own literal {literal:?}: {e}"))
}

#[test]
fn every_value_survives_its_own_engines_literal() {
    let engine = LuaEngine::new();
    assert!(engine.initialize());
    let session = "literal_round_trip";
    engine.create_session(session);

    for value in [
        ScriptValue::Bool(true),
        ScriptValue::Bool(false),
        ScriptValue::Int(42),
        ScriptValue::Int(-7),
        ScriptValue::Double(2.5),
        ScriptValue::String("plain".into()),
        // The escapes exist for these; a literal that lost one would come back
        // as a different string or fail to load at all.
        ScriptValue::String("has \"quotes\" and \\ and\na newline".into()),
    ] {
        assert_eq!(
            round_trip(&engine, session, &value),
            value,
            "value did not survive the round trip through its own literal"
        );
    }

    // Absence: Lua has one word for both of ECMAScript's, so the round trip
    // lands on Null for either — the collapse is the engine's, which is
    // exactly the kind of fact that belongs to the engine and not to the value.
    assert_eq!(
        round_trip(&engine, session, &ScriptValue::Null),
        ScriptValue::Null
    );
    assert_eq!(
        round_trip(&engine, session, &ScriptValue::Undefined),
        ScriptValue::Null
    );

    engine.destroy_session(session);
}

#[test]
fn a_structured_value_survives_as_a_table() {
    let engine = LuaEngine::new();
    assert!(engine.initialize());
    let session = "structured_round_trip";
    engine.create_session(session);

    let array = ScriptValue::Array(vec![
        ScriptValue::Int(1),
        ScriptValue::String("two".into()),
        ScriptValue::Bool(true),
    ]);
    let literal = engine.to_script_literal(&array);
    assert_eq!(
        literal, "{1, \"two\", true}",
        "a Lua sequence is braced; brackets would be a syntax error here"
    );
    assert_eq!(round_trip(&engine, session, &array), array);

    let mut map = std::collections::HashMap::new();
    map.insert("k".to_string(), ScriptValue::Int(1));
    map.insert("j".to_string(), ScriptValue::String("v".into()));
    let object = ScriptValue::Object(map);
    assert_eq!(
        engine.to_script_literal(&object),
        "{[\"j\"] = \"v\", [\"k\"] = 1}",
        "keys are sorted so equal content is equal text"
    );
    assert_eq!(round_trip(&engine, session, &object), object);

    engine.destroy_session(session);
}

/// The Lua column the sibling test quotes is this engine's, spelling for spelling.
///
/// `wire_value_is_not_engine_source.rs` cannot call this crate — the dependency
/// runs the other way — so it quotes these strings as text. This is the half
/// that makes the quote true: if the Lua spelling ever changes, this fails
/// here rather than letting a stale quote go on claiming a divergence that no
/// longer exists.
#[test]
fn the_lua_column_quoted_by_the_sibling_is_this_engines() {
    let engine = LuaEngine::new();
    assert!(engine.initialize());

    let mut single = std::collections::HashMap::new();
    single.insert("k".to_string(), ScriptValue::Int(1));

    for (value, quoted) in [
        (ScriptValue::Null, "nil"),
        (ScriptValue::Undefined, "nil"),
        (
            ScriptValue::Array(vec![ScriptValue::Int(1), ScriptValue::Int(2)]),
            "{1, 2}",
        ),
        (ScriptValue::Object(single), "{[\"k\"] = 1}"),
        (ScriptValue::Double(5.0), "5.0"),
    ] {
        assert_eq!(engine.to_script_literal(&value), quoted);
    }
}

/// An integral float is spelled with a point as source and without one on the wire.
///
/// The literal keeps the point because Lua's numeric grammar distinguishes
/// `5` from `5.0`, and the wire text drops it because ECMAScript's
/// `String(5)` is `"5"` and the receiver of a §scxml-C-2 param reads text.
/// Two renderings of one value, each right for the reader it has — which is
/// the distinction the trait method and the free function were split over.
///
/// Measured, not assumed: the value that comes back from the round trip is
/// `Int(5)`, because this engine normalizes an integral float on the way out
/// (`lua_value_to_script`) so a document comparing `x === 5` reads true — the
/// ECMAScript data model has one Number type (§scxml-B-1). The literal
/// preserved the number; the variant is the engine's business, and asserting
/// it here is how that stays deliberate rather than accidental.
#[test]
fn an_integral_float_keeps_the_point_as_source_and_drops_it_on_the_wire() {
    let engine = LuaEngine::new();
    assert!(engine.initialize());
    let session = "number_subtype";
    engine.create_session(session);

    let five = ScriptValue::Double(5.0);
    assert_eq!(engine.to_script_literal(&five), "5.0");
    assert_eq!(round_trip(&engine, session, &five), ScriptValue::Int(5));
    assert_eq!(
        sce_rust_runtime::helpers::event_data::script_value_to_wire_string(&five),
        "5"
    );
    // A fractional one keeps both its point and its variant, so the arm above
    // is normalization and not the literal losing precision.
    let half = ScriptValue::Double(2.5);
    assert_eq!(engine.to_script_literal(&half), "2.5");
    assert_eq!(round_trip(&engine, session, &half), half);

    engine.destroy_session(session);
}
