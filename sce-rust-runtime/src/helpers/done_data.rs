// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.5 / 5.7: Donedata processing helpers.
//!
//! 1:1 port of `sce/include/common/DoneDataHelper.h`. Provides JSON building
//! utilities for `_event.data` construction from `<donedata>` content and params.
//!
//! The C++ version uses `IScriptEngine` for expression evaluation. The Rust port
//! provides the same JSON serialization utilities and delegates expression
//! evaluation to the `IScriptEngine` trait from `crate::scripting`.
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): under `--features=no_std`:
//! - [`emit_content_literal`] stays available (the Rust AOT template
//!   `entry_exit_actions.rs.jinja2` emits a call to it unconditionally for
//!   `<content>literal</content>` donedata) but returns [`crate::SceString`]
//!   so it composes with the capped `EventMetadata.data` field.
//! - `build_done_data_json` and `escape_json_string` depend on
//!   `helpers::event_data` (whole-module `!no_std`-gated) and have no current
//!   consumer; they are gated to `!no_std`.

#[cfg(not(feature = "no_std"))]
use crate::helpers::event_data;
use crate::{sce_string_from_str, SceString};

/// W3C SCXML 5.5: Emit an inline `<content>` literal as `_event.data`.
///
/// 1:1 port of C++ `SCE::DoneDataHelper::emitContentLiteral`
/// (`sce/include/common/DoneDataHelper.h`). When `<content>` has no `expr`
/// attribute the spec says "the children are used as the content value" —
/// no evaluation happens and no script engine is required. The literal
/// text **is** the value.
///
/// This is the SSoT consumed by the `literal` branch of the Rust AOT
/// codegen (`tools/codegen/templates/rust/entry_exit_actions.rs.jinja2`),
/// matching the C++ `emitContentLiteral` / Go `EmitContentLiteral` /
/// Kotlin `emitContentLiteral` helpers so all four backends share one
/// semantic definition.
///
/// # Arguments
///
/// * `literal` - Inline text content from `<content>literal</content>`
///
/// # Returns
///
/// The literal as `_event.data` (raw string — no JSON quoting). Returned as
/// [`SceString`] so the no_std variant (capped `heapless::String`) composes
/// with `EventMetadata.data`.
pub fn emit_content_literal(literal: &str) -> SceString {
    sce_string_from_str(literal)
}

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
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` — delegates to
/// [`event_data::escape_json_string`] which is itself whole-module gated.
#[cfg(not(feature = "no_std"))]
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
#[cfg(not(feature = "no_std"))]
fn is_json_literal(value: &str) -> bool {
    if value == "true" || value == "false" || value == "null" {
        return true;
    }
    // Check if it's a number
    value.parse::<f64>().is_ok()
}

/// W3C SCXML 5.5: Escape JSON string (delegates to event_data module).
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` — delegates to
/// [`event_data::escape_json_string`] which is itself whole-module gated.
#[cfg(not(feature = "no_std"))]
pub fn escape_json_string(s: &str) -> String {
    event_data::escape_json_string(s)
}
