// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value that leaves the data model is the value, not the sender's language.
//
// Until 2026-08-21 `ScriptValue` — the type no engine owns — carried a method
// called `to_lua_literal`, and every caller used it for both directions:
//
//   * seeding an `<invoke>` child's datamodel, where the text IS source and
//     some engine has to parse it back, and
//   * an §scxml-C-2 HTTP `<param>`, a `targetexpr` and a `delayexpr`, where
//     the text leaves the process for a reader that is not an engine at all.
//
// Both readings were wrong in the same way. A second engine would have
// inherited Lua's spelling for the first while still compiling, and the second
// put the sender's language on the wire: the same param read `nil` from this
// backend and `` from the C++ one, `{1, 2}` here and `[1,2]` there. Four
// channels had four answers for one form-encoded field.
//
// The first direction is now gone rather than relocated. §scxml-6.4.3 asks for
// the VALUE of an `<invoke>` `<param>`, and the five other channels always
// passed it; the two that rendered source and re-read it were the two that
// lost every value Lua cannot spell — `1/0` reached the child as the text
// `inf`, which is an undeclared identifier to the engine reading it back, so
// the pair arrived as nothing. Passing the value removed the last consumer of
// an engine-owned literal, and the trait method went with it: a required
// method every future engine must answer for a use nobody has is a tax, not a
// contract. `integration_resources/invoke_param_seeds_declared_child_data/`
// is where that clause is now held, on all seven channels.
//
// What remains is the second direction, and this file is the table that pins
// it: the wire rendering is a free function with no engine in reach, so it
// cannot acquire a dialect.

use std::collections::HashMap;

use sce_rust_runtime::helpers::event_data::script_value_to_wire_string;
use sce_rust_runtime::scripting::ScriptValue;

fn object(pairs: &[(&str, ScriptValue)]) -> ScriptValue {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert((*k).to_string(), v.clone());
    }
    ScriptValue::Object(map)
}

/// Text that leaves the process is the value, not the sender's language.
#[test]
fn a_wire_param_reads_the_same_whoever_sent_it() {
    // The table C++ `ScriptResultUtils::resultToString` answers, arm for arm.
    // Every row that used to come out as Lua source is marked with what it
    // used to be, because those are the bytes that reached an HTTP peer.
    let rows: &[(ScriptValue, &str)] = &[
        (ScriptValue::Null, ""),      // was "nil"
        (ScriptValue::Undefined, ""), // was "nil"
        (ScriptValue::Bool(true), "true"),
        (ScriptValue::Bool(false), "false"),
        (ScriptValue::Int(42), "42"),
        (ScriptValue::Double(5.0), "5"), // was "5.0"
        (ScriptValue::Double(2.5), "2.5"),
        (ScriptValue::String("plain".into()), "plain"),
        // The quotes belong to the value. The old path added its own and then
        // trimmed them off, which ate these.
        (ScriptValue::String("\"quoted\"".into()), "\"quoted\""),
        (
            ScriptValue::Array(vec![ScriptValue::Int(1), ScriptValue::Int(2)]),
            "[1,2]",
        ), // was "{1, 2}"
        (
            object(&[("k", ScriptValue::String("v".into()))]),
            "{\"k\":\"v\"}",
        ), // was "{[\"k\"] = \"v\"}"
        (ScriptValue::Dom("<r><c/></r>".into()), "<r><c/></r>"),
    ];

    for (value, expected) in rows {
        assert_eq!(
            &script_value_to_wire_string(value),
            expected,
            "wire text for {:?}",
            value
        );
    }
}

/// The wire text of a string carries no quoting of its own.
///
/// This is the case where an engine's habit is easiest to acquire: as source a
/// string must be quoted or it is an identifier, and a renderer that reached
/// for an engine would quote it here too — where the quotes would be
/// characters the document never wrote.
#[test]
fn a_string_reaches_the_wire_without_quotes_of_the_senders_making() {
    assert_eq!(
        script_value_to_wire_string(&ScriptValue::String("v".into())),
        "v"
    );
    // Absence has nothing to say on the wire, where source would have had to
    // spell it.
    assert_eq!(script_value_to_wire_string(&ScriptValue::Null), "");
    assert_eq!(script_value_to_wire_string(&ScriptValue::Undefined), "");
}
