// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.5 / 5.7: Donedata processing helpers.
//!
//! 1:1 port of `sce/include/common/DoneDataHelper.h`. Provides JSON building
//! utilities for `_event.data` construction from `<donedata>` content and params.
//!
//! The C++ version uses `IScriptEngine` for expression evaluation. The Rust port
//! provides the same JSON serialization utilities and delegates expression
//! evaluation to the `IScriptEngine` trait from `crate::scripting`.

use crate::helpers::event_data;

/// W3C SCXML 5.5: Build JSON string from param name/value pairs.
///
/// Used when `<donedata>` contains `<param>` elements. Each param produces a
/// key-value pair in the JSON object.
///
/// Ports the JSON building part of C++ `DoneDataHelper::evaluateParams`.
///
/// # Arguments
///
/// * `params` - Slice of `(name, value)` pairs (already evaluated)
///
/// # Returns
///
/// JSON string like `{"Var1":"1","Var2":"hello"}`.
pub fn build_done_data_json(params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        return String::new();
    }

    let mut json = String::from("{");
    let mut first = true;

    for (name, value) in params {
        if !first {
            json.push(',');
        }
        first = false;

        json.push('"');
        json.push_str(&event_data::escape_json_string(name));
        json.push_str("\":");

        // Attempt to detect if value is already a JSON literal (number, bool, null)
        if is_json_literal(value) {
            json.push_str(value);
        } else {
            json.push('"');
            json.push_str(&event_data::escape_json_string(value));
            json.push('"');
        }
    }

    json.push('}');
    json
}

/// Check if a string value is a JSON literal that should not be quoted.
///
/// Returns `true` for: numeric values, `true`, `false`, `null`.
fn is_json_literal(value: &str) -> bool {
    if value == "true" || value == "false" || value == "null" {
        return true;
    }
    // Check if it's a number
    value.parse::<f64>().is_ok()
}

/// W3C SCXML 5.5: Escape JSON string (delegates to event_data module).
pub fn escape_json_string(s: &str) -> String {
    event_data::escape_json_string(s)
}
