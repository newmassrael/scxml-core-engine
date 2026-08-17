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
/// This is the counterpart of [`ScriptValue::to_lua_literal`], and the
/// difference between them is the whole point. A Lua literal is *source*:
/// reading it back requires an interpreter for the language the sender
/// happened to be written in, which made `_event.data` mean one thing on a
/// Lua backend and another on a JavaScript one, and made a payload from any
/// sender executable by the receiver. JSON is read by a parser.
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
