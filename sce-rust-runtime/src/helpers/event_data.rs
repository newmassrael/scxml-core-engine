// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.10 / 6.2: Event data construction helpers.
//!
//! 1:1 port of `sce/include/common/EventDataHelper.h`. Provides JSON building
//! utilities for `_event.data` construction from `<send>` params.
//!
//! The C++ version depends on `SCXMLTypes.h` and `ScriptValue`. For the Rust
//! port, we use the `ScriptValue` from `crate::scripting` and provide simpler
//! string-based JSON construction for the static code path.
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): whole-module gated to
//! `cfg(not(feature = "no_std"))` because both the input type
//! (`BTreeMap<String, Vec<String>>`) and the output type (`String`) are
//! alloc-coupled. No template currently emits calls into this helper, so
//! the no_std codegen has no need for it at present — when an MCU consumer
//! demands `<send>` param JSON construction the no_std variant lands with
//! a `heapless::String<N>` output + `&[(&str, &str)]` input under a future
//! atomic, gated on a concrete capacity requirement.

#![cfg(not(feature = "no_std"))]

use std::collections::BTreeMap;

/// W3C SCXML 5.10: Build JSON string from evaluated params.
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
