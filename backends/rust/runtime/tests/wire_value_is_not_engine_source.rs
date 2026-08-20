// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value leaves the data model two ways, and only one of them is a language.
//
// Until 2026-08-21 `ScriptValue` — the type no engine owns — carried a method
// called `to_lua_literal`, and every caller used it for both directions:
//
//   * seeding an `<invoke>` child's datamodel, where the text IS source and
//     some engine has to parse it back, and
//   * an §scxml-C-2 HTTP `<param>`, a `targetexpr` and a `delayexpr`, where
//     the text leaves the process for a reader that is not an engine at all.
//
// Both readings were wrong in the same way. A second engine (`sce-rust-quickjs`)
// would have inherited Lua's spelling for the first while still compiling, and
// the second put the sender's language on the wire: the same param read `nil`
// from this backend and `` from the C++ one, `{1, 2}` here and `[1,2]` there.
// Four channels had four answers for one form-encoded field.
//
// The split is now structural. Source is a trait method, so an engine that
// does not answer does not compile; wire text is a free function with no
// engine in reach, so it cannot acquire a dialect.
//
// This file holds the double that proves the first half — a second engine
// whose grammar is ECMAScript — and the table that proves the second.
// `backends/rust/lua/tests/engine_owns_its_literal.rs` is the sibling that
// runs the literal through the engine that has to parse it.

use std::collections::HashMap;

use sce_rust_runtime::helpers::event_data::{script_value_to_json, script_value_to_wire_string};
use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::scripting::{
    IScriptEngine, NativeMethod, ScriptError, ScriptResult, ScriptValue, SetCurrentEventArgs,
    StateQueryCallback,
};

/// A second engine, present only to be a second grammar.
///
/// It evaluates nothing: every method that would need a real interpreter
/// refuses. What it has is an opinion about how a value is spelled as source,
/// and that opinion is ECMAScript's — which is the whole point. When
/// `sce-rust-quickjs` lands it replaces this double without changing a single
/// assertion below.
struct EcmaScriptDouble;

impl EcmaScriptDouble {
    fn literal(value: &ScriptValue) -> String {
        match value {
            // The two absences ECMAScript keeps apart, which Lua cannot.
            ScriptValue::Null => "null".to_string(),
            ScriptValue::Undefined => "undefined".to_string(),
            ScriptValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            ScriptValue::Int(i) => i.to_string(),
            ScriptValue::Double(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{}", *f as i64)
                } else {
                    format!("{}", f)
                }
            }
            // An ECMAScript object literal and a JSON document agree on every
            // form this type can hold, so the JSON helper is the honest body
            // for the remaining arms.
            ScriptValue::String(_)
            | ScriptValue::Array(_)
            | ScriptValue::Object(_)
            | ScriptValue::Dom(_) => script_value_to_json(value),
        }
    }
}

impl IScriptEngine for EcmaScriptDouble {
    fn to_script_literal(&self, value: &ScriptValue) -> String {
        Self::literal(value)
    }

    fn execute_script(&self, _session_id: &str, _script: &str) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized)
    }
    fn evaluate_expression(
        &self,
        _session_id: &str,
        _expression: &str,
    ) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized)
    }
    fn validate_expression(&self, _session_id: &str, _expression: &str) -> ScriptResult<bool> {
        Err(ScriptError::NotInitialized)
    }
    fn set_variable(
        &self,
        _session_id: &str,
        _name: &str,
        _value: ScriptValue,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }
    fn get_variable(&self, _session_id: &str, _name: &str) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized)
    }
    fn set_variable_as_dom(
        &self,
        _session_id: &str,
        _name: &str,
        _xml_content: &str,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }
    fn has_variable(&self, _session_id: &str, _name: &str) -> bool {
        false
    }
    fn is_variable_pre_initialized(&self, _session_id: &str, _name: &str) -> bool {
        false
    }
    fn setup_system_variables(
        &self,
        _session_id: &str,
        _session_name: &str,
        _io_processors: &[IoProcessorDescriptor],
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }
    fn set_current_event(
        &self,
        _session_id: &str,
        _args: SetCurrentEventArgs<'_>,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }
    fn register_global_function(&self, _function_name: &str, _callback: NativeMethod) -> bool {
        false
    }
    fn bind_native_object(
        &self,
        _session_id: &str,
        _object_name: &str,
        _methods: Vec<(String, NativeMethod)>,
    ) -> bool {
        false
    }
    fn get_engine_info(&self) -> String {
        "ecmascript-double".to_string()
    }
    fn get_memory_usage(&self) -> usize {
        0
    }
    fn collect_garbage(&self) {}
    fn set_state_query_callback(&self, _session_id: &str, _callback: Option<StateQueryCallback>) {}
    fn initialize(&self) -> bool {
        true
    }
    fn shutdown(&self) {}
    fn is_initialized(&self) -> bool {
        true
    }
    fn reset(&self) {}
    fn create_session(&self, _session_id: &str) {}
    fn destroy_session(&self, _session_id: &str) {}
    fn has_session(&self, _session_id: &str) -> bool {
        false
    }
}

fn object(pairs: &[(&str, ScriptValue)]) -> ScriptValue {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert((*k).to_string(), v.clone());
    }
    ScriptValue::Object(map)
}

/// A value's source spelling follows the engine that will read it.
#[test]
fn a_second_engine_spells_a_value_its_own_way() {
    let engine = EcmaScriptDouble;

    // The arms where the two grammars are not the same text. The Lua column is
    // quoted rather than computed: `sce-rust-lua` depends on this crate, so
    // these tests cannot call it, and a second implementation of Lua's grammar
    // here would only ever agree with itself. The sibling test pins each of
    // these strings against the engine that has to parse it, so a change to
    // the Lua spelling breaks there and a lie here is caught there.
    let diverging: &[(ScriptValue, &str, &str)] = &[
        (ScriptValue::Null, "null", "nil"),
        (ScriptValue::Undefined, "undefined", "nil"),
        (
            ScriptValue::Array(vec![ScriptValue::Int(1), ScriptValue::Int(2)]),
            "[1,2]",
            "{1, 2}",
        ),
        (
            object(&[("k", ScriptValue::Int(1))]),
            "{\"k\":1}",
            "{[\"k\"] = 1}",
        ),
        (ScriptValue::Double(5.0), "5", "5.0"),
    ];

    for (value, ecmascript, lua) in diverging {
        // A defaulted trait method — or the inherent `to_lua_literal` this
        // replaced — makes these equal, which is the regression being fenced.
        assert_ne!(
            ecmascript, lua,
            "the table's own two columns must differ for {value:?}"
        );
        assert_eq!(
            &engine.to_script_literal(value),
            ecmascript,
            "second engine's literal for {value:?}"
        );
    }

    // Where the grammars agree they agree — a number and a boolean are spelled
    // the same in both — so the divergence above is about syntax and not about
    // the double being gratuitously different.
    for same in [ScriptValue::Int(7), ScriptValue::Bool(true)] {
        assert_eq!(
            engine.to_script_literal(&same),
            script_value_to_wire_string(&same)
        );
    }
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

/// The wire rendering is not reachable from an engine, so it cannot pick one up.
#[test]
fn the_two_directions_do_not_answer_the_same() {
    let engine = EcmaScriptDouble;
    // A string is the case where the difference is easiest to lose: as source
    // it must be quoted or it is an identifier; as wire text the quotes would
    // be characters the document never wrote.
    let s = ScriptValue::String("v".into());
    assert_eq!(engine.to_script_literal(&s), "\"v\"");
    assert_eq!(script_value_to_wire_string(&s), "v");

    // Absence is the other: source has to spell it, the wire has nothing to say.
    assert_eq!(engine.to_script_literal(&ScriptValue::Null), "null");
    assert_eq!(script_value_to_wire_string(&ScriptValue::Null), "");
}
