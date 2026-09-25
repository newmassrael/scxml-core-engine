// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-6.2.4 / §scxml-6.4.1: writing a generated id to an `idlocation`.
//!
//! An `idlocation` is a location expression, so the write is an assignment
//! and takes the path `<assign>` takes: a member path such as `obj.slot`
//! lands, and a location that cannot be assigned is a failure the caller
//! reports as `error.execution` (§scxml-5.9.2). Binding the text as a global
//! name, as the invoke sites did, created a variable literally called
//! `obj.slot` and reported success.
//!
//! The id is a run-time value — an SCXML child's is `state.<instance>.<id>` —
//! so it is quoted here, when it exists, not at code generation.

use crate::scripting::IScriptEngine;

/// `text` as a Lua string literal.
///
/// Not Rust's `{:?}`, which spells non-ASCII text as `\u{..}`: Lua reads that
/// escape differently, so the stored id would not be the one generated.
pub fn lua_string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Assign `id` to the location `lua_location` names, already lowered by the
/// generator (`to_lua_location`). Returns whether the assignment took; the
/// caller raises `error.execution` when it did not.
pub fn store_id_in_location(
    engine: &dyn IScriptEngine,
    session_id: &str,
    lua_location: &str,
    id: &str,
) -> bool {
    engine
        .execute_script(
            session_id,
            &format!("{lua_location} = {}", lua_string_literal(id)),
        )
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_keeps_what_lua_would_misread() {
        assert_eq!(lua_string_literal("a.b"), "\"a.b\"");
        assert_eq!(lua_string_literal("q\"\\\n"), "\"q\\\"\\\\\\n\"");
        // Non-ASCII passes through as UTF-8 bytes Lua keeps verbatim, not as
        // a `\u{..}` escape.
        assert_eq!(lua_string_literal("é"), "\"é\"");
    }
}
