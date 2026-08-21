// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-5.10 / 6.2: Event data construction helpers.
//!
//! 1:1 port of `sce/include/common/EventDataHelper.h`. Provides JSON building
//! utilities for `_event.data` construction from `<send>` params.
//!
//! The C++ version depends on `SCXMLTypes.h` and `ScriptValue`. For the Rust
//! port, we use the `ScriptValue` from `crate::scripting` and provide simpler
//! string-based JSON construction for the static code path.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): whole-module gated to
//! `cfg(not(feature = "no_std"))` because both the input type
//! (`BTreeMap<String, Vec<String>>`) and the output type (`String`) are
//! alloc-coupled. No template currently emits calls into this helper, so
//! the no_std codegen has no need for it at present — when an MCU consumer
//! demands `<send>` param JSON construction the no_std variant lands with
//! a `heapless::String<N>` output + `&[(&str, &str)]` input under a future
//! atomic, gated on a concrete capacity requirement.

#![cfg(not(feature = "no_std"))]

use crate::scripting::ScriptValue;
use std::collections::BTreeMap;

/// A value that leaves the ECMAScript data model, as JSON.
///
/// 1:1 port of the C++ `scriptValueToJson` static in
/// `sce/src/common/EventDataHelper.cpp`. The clause cited in the body names
/// JSON as the serialization for data that leaves the data model — it is what
/// the BasicHTTP Event I/O Processor sends — and an event payload leaves it by
/// definition: the receiver may be a different session, a different process,
/// or a different backend, and the only thing all of them can read is data.
///
/// This is the counterpart of [`IScriptEngine::to_script_literal`], and the
/// difference between them is the whole point. An engine literal is *source*:
/// reading it back requires an interpreter for the language the sender
/// happened to be written in, which made `_event.data` mean one thing on a
/// Lua backend and another on a JavaScript one, and made a payload from any
/// sender executable by the receiver. JSON is read by a parser.
///
/// [`IScriptEngine::to_script_literal`]: crate::scripting::IScriptEngine::to_script_literal
///
/// Object keys are sorted. `HashMap` iteration order is not stable between
/// runs, and the wire form has to be byte-identical for equal content —
/// the committed-tree sweep compares generated output, and a payload that
/// reordered itself per run would fail it for no semantic reason.
pub fn script_value_to_json(value: &ScriptValue) -> String {
    // §scxml-B-2-9: a value that has to leave the ECMAScript data model is
    // serialized to JSON, which reconstructs it in full rather than falling
    // back to a lossy platform format.
    match value {
        // JSON has no `undefined`; the C++ port maps both to null, so a
        // round trip yields `null` rather than restoring the distinction.
        ScriptValue::Null | ScriptValue::Undefined => "null".to_string(),
        ScriptValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        ScriptValue::Int(i) => i.to_string(),
        ScriptValue::Double(f) => {
            if f.is_nan() || f.is_infinite() {
                // RFC 8259 has no spelling for either.
                "null".to_string()
            } else if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        ScriptValue::String(s) => format!("\"{}\"", escape_json_string(s)),
        ScriptValue::Array(items) => {
            let parts: Vec<String> = items.iter().map(script_value_to_json).collect();
            format!("[{}]", parts.join(","))
        }
        ScriptValue::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "\"{}\":{}",
                        escape_json_string(k),
                        script_value_to_json(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        // The data model reads XML into a DOM at the *receiving* end, from
        // the document text — so a DOM that reaches here crosses as that text.
        ScriptValue::Dom(xml) => format!("\"{}\"", escape_json_string(xml)),
    }
}

/// A single value as the text a form-encoded §scxml-C-2 param carries.
///
/// The BasicHTTP Event I/O Processor sends each `<param>` as one `name=value`
/// pair, so the value crosses as *text* and the receiving end hands that text
/// to `_event.data` — no script engine reads it at either end. That is why
/// this is neither of the two serializations beside it: [`script_value_to_json`]
/// would wrap a string in quotes that are not part of it, and an engine
/// literal (`IScriptEngine::to_script_literal`) would put the sender's
/// *language* on the wire, so one value would read `nil` from a Lua-backed
/// sender and `null` from a JavaScript-backed one.
///
/// The rendering is ECMAScript's `String(value)` — §scxml-B-1 makes the data
/// model ECMAScript, so its `ToString` is what a number or a boolean means as
/// text — with the two amendments C++ `ScriptResultUtils::resultToString`
/// already made:
///
/// - `null` and `undefined` render empty rather than as the words. §scxml-C-1
///   reads a value that is not there as the empty string, and a param
///   carrying the four letters `null` could not be told from one carrying
///   the word.
/// - a structured value renders as JSON, because a receiver that is not a
///   script engine has no other reading of it. This is the one arm that
///   delegates to [`script_value_to_json`].
///
/// Two arms are ECMAScript's rather than that helper's, and the difference is
/// recorded rather than hidden: a non-finite number spells `NaN` / `Infinity`
/// here where the C++ helper hands the value to an `ostringstream` and gets
/// `nan` / `inf`, and a DOM crosses as its document text where the C++ helper
/// would reach its `JSON.stringify` fallback. Both are the ECMAScript answer
/// (§scxml-B-1) and neither is reached by any fixture in the corpus, so the
/// C++ side is a debt to align rather than a divergence to copy.
pub fn script_value_to_wire_string(value: &ScriptValue) -> String {
    match value {
        // §scxml-C-1: a value that is not there is the empty string — on the
        // wire as in a target expression, the same reading C++ gives both.
        ScriptValue::Null | ScriptValue::Undefined => String::new(),
        ScriptValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        ScriptValue::Int(i) => i.to_string(),
        ScriptValue::Double(f) => {
            if f.is_nan() {
                "NaN".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
            } else if f.fract() == 0.0 && f.abs() < 1e15 {
                // ECMAScript `String(5)` is "5". A `.0` tail is the Rust
                // spelling of the number, not the document's.
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        // Already text. Quoting it here would deliver characters the document
        // never wrote, and the trim that used to undo such quotes ate the
        // ones the value itself carried.
        ScriptValue::String(s) => s.clone(),
        // The receiving end reads XML from document text (§scxml-B-2), so a
        // DOM crosses as that text rather than as a quoted JSON string.
        ScriptValue::Dom(xml) => xml.clone(),
        ScriptValue::Array(_) | ScriptValue::Object(_) => script_value_to_json(value),
    }
}

/// A `<send>`'s evaluated params, as the JSON `_event.data` is.
///
/// 1:1 port of C++ `EventDataHelper::buildJsonFromTypedParams`. One
/// occurrence of a name is the value itself; more than one is an Array of
/// them in document order, because W3C test178 sends a name twice and
/// requires both values delivered while an object cannot hold one name
/// twice.
///
/// The type is preserved, which is the reason this takes [`ScriptValue`]
/// rather than strings: a receiver comparing `_event.data.value === 42`
/// reads false against the string `"42"`.
pub fn build_json_from_typed_params(params: &BTreeMap<String, Vec<ScriptValue>>) -> String {
    // §scxml-6.2: the `<param>` elements a `<send>` carries become the data
    // the receiving event exposes, evaluated at send time.
    let mut parts: Vec<String> = Vec::new();
    for (name, values) in params {
        if values.is_empty() {
            continue;
        }
        let published = if values.len() == 1 {
            script_value_to_json(&values[0])
        } else {
            let items: Vec<String> = values.iter().map(script_value_to_json).collect();
            format!("[{}]", items.join(","))
        };
        parts.push(format!("\"{}\":{}", escape_json_string(name), published));
    }
    format!("{{{}}}", parts.join(","))
}

/// §scxml-5.10: Build JSON string from evaluated params.
///
/// Supports duplicate param names (W3C test 178) by storing multiple values
/// per key as a JSON array.
///
/// Ports C++ `EventDataHelper::buildJsonFromParams`.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use sce_rust_runtime::helpers::event_data::build_json_from_params;
///
/// let mut params = BTreeMap::new();
/// params.insert("name".to_string(), vec!["value".to_string()]);
/// assert_eq!(build_json_from_params(&params), r#"{"name":"value"}"#);
/// ```
pub fn build_json_from_params(params: &BTreeMap<String, Vec<String>>) -> String {
    if params.is_empty() {
        return "{}".to_string();
    }

    let mut json = String::from("{");
    let mut first = true;

    for (name, values) in params {
        if !first {
            json.push(',');
        }
        first = false;

        json.push('"');
        json.push_str(&escape_json_string(name));
        json.push_str("\":");

        if values.len() == 1 {
            // Single value
            json.push('"');
            json.push_str(&escape_json_string(&values[0]));
            json.push('"');
        } else {
            // W3C Test 178: Multiple values as array
            json.push('[');
            for (i, val) in values.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push('"');
                json.push_str(&escape_json_string(val));
                json.push('"');
            }
            json.push(']');
        }
    }

    json.push('}');
    json
}

/// Escape special characters for JSON string values.
///
/// Ports C++ `DoneDataHelper::escapeJsonString`.
pub fn escape_json_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0008}' => escaped.push_str("\\b"), // backspace
            '\u{000C}' => escaped.push_str("\\f"), // form feed
            other => escaped.push(other),
        }
    }
    escaped
}

/// Build a simple JSON object from key-value pairs (all string values).
///
/// Convenience helper for generated code that constructs `_event.data` from
/// static `<param>` elements.
pub fn build_json_object(pairs: &[(&str, &str)]) -> String {
    if pairs.is_empty() {
        return "{}".to_string();
    }

    let mut json = String::from("{");
    for (i, (key, value)) in pairs.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push('"');
        json.push_str(&escape_json_string(key));
        json.push_str("\":\"");
        json.push_str(&escape_json_string(value));
        json.push('"');
    }
    json.push('}');
    json
}
