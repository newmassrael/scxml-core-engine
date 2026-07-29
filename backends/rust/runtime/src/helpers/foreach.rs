// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-4.6: Foreach loop helpers (static variant).
//!
//! 1:1 port of `sce/include/core/ForeachHelper.h` (static subset). Provides
//! variable name validation and the static foreach iteration pattern.
//!
//! The script-engine foreach variant (array evaluation, variable setting) is
//! emitted directly by generated code via the `crate::scripting` engine traits
//! (`tools/codegen/templates/rust/actions/foreach.rs.jinja2`). This module
//! provides the W3C-compliant validation and iteration structure that
//! generated code uses with pre-evaluated arrays.

/// §scxml-B-2: Validate that a variable name is a legal ECMAScript identifier.
///
/// Ports C++ `ForeachHelper::isLegalVariableName`.
pub fn is_legal_variable_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must not be quoted
    let first_byte = name.as_bytes()[0];
    if first_byte == b'\'' || first_byte == b'"' {
        return false;
    }

    // Must start with letter, underscore, or dollar sign
    if !first_byte.is_ascii_alphabetic() && first_byte != b'_' && first_byte != b'$' {
        return false;
    }

    // Must contain only alphanumeric, underscore, or dollar sign
    name.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

/// §scxml-4.6: Execute a static foreach loop with a body closure.
///
/// Iterates over a pre-evaluated array of items, calling `set_variables` to
/// bind the item and index variables, then `execute_body` for each iteration.
///
/// This is the static variant -- array values are already known at call time
/// (no script engine evaluation needed). For script-engine-based foreach, use
/// `ForeachHelper::executeForeachWithActions` with the script engine directly.
///
/// # §scxml-4.6 Compliance
///
/// "If the evaluation of any child element of foreach causes an error, the
/// processor MUST cease execution of the foreach element and the block that
/// contains it."
///
/// Returns `true` if all iterations succeeded, `false` if stopped by error.
pub fn execute_static_foreach<T>(
    items: &[T],
    mut set_variables: impl FnMut(&T, usize) -> bool,
    mut execute_body: impl FnMut(usize) -> bool,
) -> bool {
    for (i, item) in items.iter().enumerate() {
        // Set item and index variables
        if !set_variables(item, i) {
            return false;
        }

        // Execute body actions
        // §scxml-4.6: If body returns false (error), stop loop
        if !execute_body(i) {
            return false;
        }
    }

    true
}
