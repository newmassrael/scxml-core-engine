// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.2.2 / 5.3: Datamodel initialization helpers.
//!
//! 1:1 port of `sce/include/common/DataModelInitHelper.h`. Provides utilities
//! for initializing datamodel variables from content, src, and expr attributes.
//!
//! The C++ version delegates expression evaluation to `IScriptEngine`. The Rust
//! port provides the initialization orchestration logic; actual script engine
//! calls happen through the `IScriptEngine` trait from `crate::scripting`.

use std::path::{Path, PathBuf};

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
pub fn resolve_src_path(src: &str, base_path: &str) -> PathBuf {
    let file_path = src.strip_prefix("file:").unwrap_or(src);

    let path = Path::new(file_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(base_path).join(file_path)
    }
}
