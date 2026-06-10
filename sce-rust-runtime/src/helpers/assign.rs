// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.3 / 5.4: Assignment helpers (static-value variant).
//!
//! Merged port of:
//! - `sce/include/common/AssignHelper.h` (location validation)
//! - `sce/include/common/AssignmentExecutionHelper.h` (system variable detection)
//!
//! Only the static-value assignment path is ported here (no script engine
//! evaluation). Script-engine assignment (C++ `executeAssignment`) is emitted
//! directly by generated code via the `crate::scripting` engine traits
//! (`tools/codegen/templates/rust/actions/assign.rs.jinja2`).

/// W3C SCXML B.2: System variables that are read-only.
const SYSTEM_VARIABLES: &[&str] = &["_sessionid", "_event", "_name", "_ioprocessors"];

/// W3C SCXML 5.3/5.4: Validate assignment location.
///
/// Returns `true` if the location is valid and writable, `false` if empty
/// or a read-only system variable.
///
/// Ports C++ `AssignHelper::isValidLocation`.
pub fn is_valid_location(location: &str) -> bool {
    // W3C SCXML 5.3/5.4: Empty location is invalid
    if location.is_empty() {
        return false;
    }

    // W3C SCXML B.2: System variables are read-only
    if SYSTEM_VARIABLES.contains(&location) {
        return false;
    }

    true
}

/// W3C SCXML 5.3/5.4: Get error message for an invalid location.
///
/// Ports C++ `AssignHelper::getInvalidLocationErrorMessage`. Returns
/// [`crate::SceString`] (= `String` under std, capped `heapless::String`
/// under no_std).
///
/// Watching-zenoh RFC §5.J.2: the format-and-return shape is preserved under
/// no_std via `core::fmt::Write` into a `heapless::String`.
pub fn get_invalid_location_error(location: &str) -> crate::SceString {
    if location.is_empty() {
        crate::sce_string_from_str("Assignment location cannot be empty")
    } else if SYSTEM_VARIABLES.contains(&location) {
        format_location_error("Cannot assign to read-only system variable: ", location)
    } else {
        format_location_error("Invalid assignment location: ", location)
    }
}

/// Build `{prefix}{location}` into a fresh [`crate::SceString`].
///
/// Single formatter shared by the two non-empty branches of
/// [`get_invalid_location_error`] so the std/no_std split lives in one place.
fn format_location_error(prefix: &str, location: &str) -> crate::SceString {
    #[cfg(not(feature = "no_std"))]
    {
        format!("{}{}", prefix, location)
    }
    #[cfg(feature = "no_std")]
    {
        use core::fmt::Write;
        let mut s = crate::SceString::new();
        let _ = write!(&mut s, "{}{}", prefix, location);
        s
    }
}

/// W3C SCXML 5.10: Check if an expression references a system variable.
///
/// System variable references (`_event`, `_sessionid`, etc.) require special
/// handling to preserve JavaScript object reference semantics.
///
/// Ports C++ `AssignmentExecutionHelper::isSystemVariableReference`.
pub fn is_system_variable_reference(expr: &str) -> bool {
    matches!(
        expr,
        "_sessionid" | "_event" | "_name" | "_ioprocessors" | "_x"
    )
}

/// Check if a location is a simple variable name (no dots, brackets, etc.).
///
/// Simple variables use `setVariable`, complex paths use `executeScript`.
/// Ports the regex check in C++ `AssignmentExecutionHelper::executeAssignment`.
pub fn is_simple_variable_name(location: &str) -> bool {
    if location.is_empty() {
        return false;
    }

    let bytes = location.as_bytes();

    // Must start with letter, underscore, or dollar
    if !bytes[0].is_ascii_alphabetic() && bytes[0] != b'_' && bytes[0] != b'$' {
        return false;
    }

    // Must contain only alphanumeric, underscore, or dollar
    bytes
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}
