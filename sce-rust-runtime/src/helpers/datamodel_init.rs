// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.2.2 / 5.3: Datamodel initialization helpers.
//!
//! 1:1 port of `sce/include/common/DataModelInitHelper.h`. Provides utilities
//! for initializing datamodel variables from content, src, and expr attributes.
//!
//! The C++ version delegates expression evaluation to `IScriptEngine`. The Rust
//! port provides the initialization orchestration logic; actual script engine
//! calls happen through the `IScriptEngine` trait from `crate::scripting`.
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): the no_std statechart variant
//! has zero `alloc` dependency. Filesystem-coupled helpers (`Path`/`PathBuf`
//! types, `std::env::current_exe`, `std::fs::read_to_string`) are gated to
//! `cfg(not(feature = "no_std"))`. Author-side `<data src="...">` is rejected
//! up-front by `validate_no_std_compatibility` (diagnostic
//! `codegen/no-std-fs-load-not-supported`), so under `--no-std` the
//! initialize-from-src path is unreachable.

#[cfg(not(feature = "no_std"))]
use std::path::{Path, PathBuf};

#[cfg(not(feature = "no_std"))]
use crate::scripting::{IScriptEngine, ScriptValue};

/// W3C SCXML 5.2.2: Check if expression is a JavaScript function literal.
///
/// Function expressions (`function() {...}` or `() => ...`) must preserve
/// function type and not be evaluated as regular expressions.
///
/// Ports C++ `DataModelInitHelper::isFunctionExpression`.
pub fn is_function_expression(expr: &str) -> bool {
    let trimmed = expr.trim();
    trimmed.starts_with("function") || trimmed.starts_with("() =>") || trimmed.starts_with("()")
}

/// Resolve a relative path against the executable's directory.
///
/// AOT tests need location-independent base path resolution. Converts a
/// relative path to absolute based on the executable's location.
///
/// Ports C++ `DataModelInitHelper::resolveExecutableBasePath`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `PathBuf` and
/// `std::env::current_exe` are alloc-coupled.
#[cfg(not(feature = "no_std"))]
pub fn resolve_executable_base_path(relative_path: &str) -> PathBuf {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(exe_dir) = exe_path.parent() {
                exe_dir.join(relative_path)
            } else {
                PathBuf::from(relative_path)
            }
        }
        Err(_) => PathBuf::from(relative_path),
    }
}

/// Check if content string is XML (starts with `<`).
///
/// W3C SCXML B.2: XML content requires DOM conversion.
pub fn is_xml_content(content: &str) -> bool {
    content.trim_start().starts_with('<')
}

/// Resolve a `src` attribute URL to a file path.
///
/// Strips the `file:` prefix if present and resolves relative to `base_path`.
///
/// W3C SCXML 5.2.2: External source loading via `src` attribute.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `Path`/`PathBuf` are
/// alloc-coupled. Under `--no-std` the codegen-time validator rejects
/// `<data src="...">` so this helper is never emitted into generated code.
#[cfg(not(feature = "no_std"))]
pub fn resolve_src_path(src: &str, base_path: &str) -> PathBuf {
    let file_path = src.strip_prefix("file:").unwrap_or(src);

    let path = Path::new(file_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(base_path).join(file_path)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// W3C SCXML 5.2.2: Variable initialization orchestration
// Matches C++ DataModelInitHelper (Zero Duplication: shared by codegen templates)
// ═══════════════════════════════════════════════════════════════════════════

/// W3C SCXML B.2: Evaluate expression and set variable, with string fallback.
///
/// Tries `evaluate_expression` first; on failure, falls back to setting the
/// variable as a plain string. Used for content initialization where the value
/// may be a valid expression or just text.
///
/// Ports the evaluate-then-fallback pattern from C++ `DataModelInitHelper::initializeVariable`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because [`IScriptEngine`] is
/// part of `crate::scripting`, which is whole-module gated under `--no-std`
/// (the `codegen/no-std-script-not-supported` validator rejects `<script>`
/// up-front so this helper is unreachable from generated no_std code).
#[cfg(not(feature = "no_std"))]
pub fn eval_or_set_string(
    se: &dyn IScriptEngine,
    sid: &str,
    var_id: &str,
    expression: &str,
    fallback: &str,
) -> Result<(), String> {
    match se.evaluate_expression(sid, expression) {
        Ok(val) => se.set_variable(sid, var_id, val).map_err(|e| e.to_string()),
        Err(_) => {
            // W3C SCXML B.2 test 446: Try JSON.parse for file-loaded JSON content
            // External files may contain JSON ([1,2,3] or {"key":"val"}) which isn't
            // valid Lua. The C++ engine handles this via its runtime ES→Lua transformer;
            // in Rust we use JSON.parse (registered as Lua builtin) instead.
            let trimmed = expression.trim();
            if trimmed.starts_with('[') || trimmed.starts_with('{') {
                let json_expr = format!("JSON.parse([==[{}]==])", trimmed);
                if let Ok(val) = se.evaluate_expression(sid, &json_expr) {
                    return se.set_variable(sid, var_id, val).map_err(|e| e.to_string());
                }
            }
            // W3C SCXML B.2 test 558: Fallback to string when expression evaluation fails
            se.set_variable(sid, var_id, ScriptValue::String(fallback.to_string()))
                .map_err(|e| e.to_string())
        }
    }
}

/// W3C SCXML 5.2.2: Initialize variable from runtime content string.
///
/// Three-step process matching C++ `DataModelInitHelper::initializeVariable`:
/// 1. Empty content -> set to null
/// 2. XML detected (starts with `<`) -> `set_variable_as_dom()`
/// 3. Otherwise -> `evaluate_expression()` with whitespace-normalized string fallback
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` for the same reason as
/// [`eval_or_set_string`] — the [`IScriptEngine`] dependency.
#[cfg(not(feature = "no_std"))]
pub fn initialize_variable(
    se: &dyn IScriptEngine,
    sid: &str,
    var_id: &str,
    content: &str,
) -> Result<(), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return se
            .set_variable(sid, var_id, ScriptValue::Null)
            .map_err(|e| e.to_string());
    }
    // W3C SCXML B.2: Detect XML content and create DOM object
    if is_xml_content(trimmed) {
        return se
            .set_variable_as_dom(sid, var_id, trimmed)
            .map_err(|e| e.to_string());
    }
    // Try evaluating as expression, fall back to whitespace-normalized string
    let normalized: String = trimmed.split_whitespace().collect::<Vec<&str>>().join(" ");
    eval_or_set_string(se, sid, var_id, trimmed, &normalized)
}

/// W3C SCXML 5.2.2: Load variable from external file via `src` attribute.
///
/// Strips `file:` prefix, resolves path relative to `base_path`, reads file
/// content, then delegates to [`initialize_variable`].
///
/// Ports C++ `DataModelInitHelper::initializeVariableFromSrc`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `std::fs::read_to_string`
/// is OS-coupled and `PathBuf` is alloc-coupled. Under `--no-std` the codegen-time
/// validator rejects `<data src="...">` so this helper is never emitted into
/// generated code.
#[cfg(not(feature = "no_std"))]
pub fn initialize_variable_from_src(
    se: &dyn IScriptEngine,
    sid: &str,
    var_id: &str,
    src: &str,
    base_path: &str,
) -> Result<(), String> {
    let path = resolve_src_path(src, base_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read src file '{}': {}", path.display(), e))?;
    initialize_variable(se, sid, var_id, &content)
}

/// W3C SCXML 5.3: Initialize variable from `expr` attribute (no fallback).
///
/// Evaluates the expression and sets the variable. Returns error on failure
/// (W3C SCXML 5.3: raises `error.execution`). Unlike [`initialize_variable`],
/// there is no string fallback — expr failures are always errors.
///
/// Ports C++ `DataModelInitHelper::initializeVariableFromExpr`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` for the same reason as
/// [`eval_or_set_string`] — the [`IScriptEngine`] dependency.
#[cfg(not(feature = "no_std"))]
pub fn initialize_variable_from_expr(
    se: &dyn IScriptEngine,
    sid: &str,
    var_id: &str,
    expr: &str,
) -> Result<(), String> {
    match se.evaluate_expression(sid, expr) {
        Ok(val) => se.set_variable(sid, var_id, val).map_err(|e| e.to_string()),
        Err(e) => Err(format!(
            "Failed to evaluate expr for '{}': {}",
            var_id, e
        )),
    }
}
