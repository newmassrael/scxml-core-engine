// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Jinja2 template filters — multi-language filter registry.
// Rust, C++, and Kotlin filters registered with minijinja for template rendering.

use minijinja::Value;
use regex::Regex;
use std::sync::LazyLock;

// ── Compiled regex patterns (compiled once, reused across calls) ──

/// Splits on dot, underscore, or hyphen — used for PascalCase, camelCase, event variants.
static RE_DELIMITERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[._\-]").unwrap());

/// Splits on underscore or hyphen only (no dot) — used for state variants, state/event class names.
static RE_WORD_DELIMITERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[_\-]").unwrap());

/// Matches C++ `isStateActive("stateId")` calls for Rust transformation.
static RE_IS_STATE_ACTIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"isStateActive\("([^"]+)"\)"#).unwrap());

/// Matches one or more whitespace characters for normalization.
static RE_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Capitalize the first character of a string.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Rust 2021 edition reserved keywords — must be escaped with `r#` prefix
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "async", "await", "dyn",
    // Reserved for future use
    "abstract", "become", "box", "do", "final", "macro", "override", "priv",
    "typeof", "unsized", "virtual", "yield", "try", "union",
];

/// Register all Rust-specific filters on the minijinja environment.
pub fn register_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_rust_type", to_rust_type);
    env.add_filter("escape_rust", escape_rust);
    env.add_filter("to_rust_string_expr", to_rust_string_expr);
    env.add_filter("to_event_variant", to_event_variant);
    env.add_filter("to_state_variant", to_state_variant);
    env.add_filter("to_rust_literal", to_rust_literal);
    env.add_filter("escape_keyword", escape_keyword);
    env.add_filter("to_in_predicate_rust", to_in_predicate_rust);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("to_machine_name", to_pascal_case);
    // ECMAScript→Lua transformation filters
    env.add_filter("to_lua_expr", to_lua_expr);
    env.add_filter("to_lua_guard", to_lua_guard);
    env.add_filter("to_lua_script", to_lua_script);
    // Cross-engine compatibility filters (replace Python-specific string methods)
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
}

/// Convert identifier to snake_case for Rust function/variable/module names.
pub fn to_snake_case(name: String) -> String {
    if name.is_empty() {
        return "empty".to_string();
    }
    let name = name.replace(['.', '-'], "_");
    // Insert underscore before uppercase letters preceded by lowercase/digit
    // (Rust regex doesn't support lookaround, so we build manually)
    let mut result = String::with_capacity(name.len() + 4);
    let chars: Vec<char> = name.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_ascii_uppercase() {
            let prev = chars[i - 1];
            if prev.is_ascii_lowercase() || prev.is_ascii_digit() {
                result.push('_');
            }
        }
        result.push(ch);
    }
    result.to_lowercase()
}

/// Convert identifier to PascalCase for Rust struct/enum/variant names.
pub fn to_pascal_case(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    RE_DELIMITERS
        .split(&name)
        .map(|p| if p.is_empty() { String::new() } else { capitalize_first(p) })
        .collect()
}

/// Map SCXML variable type to Rust type.
fn to_rust_type(var_type: String) -> String {
    match var_type.as_str() {
        "int" => "i64".to_string(),
        "string" => "String".to_string(),
        "bool" => "bool".to_string(),
        _ => "sce_rust_runtime::ScriptValue".to_string(),
    }
}

/// Escape characters for Rust string literals.
pub fn escape_rust(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Convert SCXML string expression to Rust string expression.
fn to_rust_string_expr(expr: String) -> String {
    if expr.is_empty() {
        return "\"\"".to_string();
    }
    let stripped = expr.trim();
    if stripped.len() >= 2 && stripped.starts_with('\'') && stripped.ends_with('\'') {
        let inner = &stripped[1..stripped.len() - 1];
        let inner = inner.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{inner}\"")
    } else {
        expr
    }
}

/// Convert dot-separated SCXML event name to Rust enum variant PascalCase.
/// Identical to to_pascal_case — both split on dot/underscore/hyphen delimiters.
pub fn to_event_variant(name: String) -> String {
    to_pascal_case(name)
}

/// Convert SCXML state ID to Rust enum variant PascalCase.
/// State variants only split on _ and -, NOT on . (unlike event variants).
pub fn to_state_variant(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    RE_WORD_DELIMITERS
        .split(&name)
        .map(|p| if p.is_empty() { String::new() } else { capitalize_first(p) })
        .collect()
}

/// Render a value as a Rust literal expression.
///
/// NOTE: bool is checked before i64 intentionally. minijinja stores bools and
/// integers as distinct types, so `i64::try_from(true)` fails. If a future
/// minijinja version changes this behavior, the ordering here prevents `0`/`1`
/// from being misinterpreted as `false`/`true`.
fn to_rust_literal(value: Value) -> String {
    if value.is_none() || value.is_undefined() {
        return "None".to_string();
    }
    if let Ok(b) = bool::try_from(value.clone()) {
        return if b { "true" } else { "false" }.to_string();
    }
    if let Ok(i) = i64::try_from(value.clone()) {
        return i.to_string();
    }
    if let Ok(f) = f64::try_from(value.clone()) {
        return format!("{f}_f64");
    }
    if let Some(s) = value.as_str() {
        let escaped = escape_rust(s.to_string());
        return format!("\"{escaped}\".to_string()");
    }
    "Default::default()".to_string()
}

/// Prefix Rust keywords with `r#` to make them valid identifiers.
fn escape_keyword(name: String) -> String {
    if RUST_KEYWORDS.contains(&name.as_str()) {
        format!("r#{name}")
    } else {
        name
    }
}

/// Transform C++ In() predicate code to Rust.
fn to_in_predicate_rust(cond_cpp: String) -> String {
    if cond_cpp.is_empty() {
        return String::new();
    }
    let result = cond_cpp.replace("this->", "");
    RE_IS_STATE_ACTIVE
        .replace_all(&result, r#"self.is_state_active("$1")"#)
        .to_string()
}

/// W3C SCXML B.2: Normalize whitespace.
fn normalize_ws(text: String) -> String {
    RE_WHITESPACE.replace_all(text.trim(), " ").to_string()
}

// ── ECMAScript→Lua transformation filters ─────────────────────────

fn to_lua_expr(expr: String) -> String {
    if expr.is_empty() {
        return String::new();
    }
    crate::lua_transformer::transform_expression(
        expr.trim_end_matches(';'),
        crate::lua_transformer::ExpressionContext::General,
    )
}

fn to_lua_guard(expr: String) -> String {
    if expr.is_empty() {
        return "true".to_string();
    }
    crate::lua_transformer::transform_expression(
        expr.trim_end_matches(';'),
        crate::lua_transformer::ExpressionContext::Guard,
    )
}

fn to_lua_script(script: String) -> String {
    if script.is_empty() {
        return String::new();
    }
    crate::lua_transformer::transform_script(&script)
}

// ── Cross-engine compatibility filters ────────────────────────────

/// Filter an iterable of serialised `Invoke` sum values by variant discriminator.
///
/// Our `Invoke` enum is serialised internally-tagged (`#[serde(tag = "kind")]`),
/// so each entry has a `kind` field of `"Scxml" | "Hybrid" | "MeshRpc"`. This
/// helper keeps only the entries whose `kind` matches — giving templates a
/// short, named filter (e.g. `state.invokes | scxml`) instead of repeating
/// `selectattr('kind', 'equalto', 'Scxml') | list` at every use site.
fn filter_invokes_by_kind(value: Value, want_kind: &str) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        match item.get_attr("kind") {
            Ok(kind_val) => {
                if let Some(k) = kind_val.as_str() {
                    if k == want_kind {
                        out.push(item.clone());
                    }
                }
            }
            // Non-Invoke items simply fall through — filter returns only matches.
            Err(_) => {}
        }
    }
    Ok(Value::from(out))
}

/// Keep only `Invoke::Scxml` entries.
fn filter_scxml(value: Value) -> Result<Value, minijinja::Error> {
    filter_invokes_by_kind(value, "Scxml")
}

/// Keep only `Invoke::Hybrid` entries.
fn filter_hybrid(value: Value) -> Result<Value, minijinja::Error> {
    filter_invokes_by_kind(value, "Hybrid")
}

/// Keep `Invoke::Scxml` and `Invoke::Hybrid` — the W3C SCXML-session kinds.
/// Mirrors the legacy `static_invokes or hybrid_invokes` guard used throughout
/// the codegen templates.
fn filter_scxml_family(value: Value) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        if let Ok(kind_val) = item.get_attr("kind") {
            if let Some(k) = kind_val.as_str() {
                if k == "Scxml" || k == "Hybrid" {
                    out.push(item.clone());
                }
            }
        }
    }
    Ok(Value::from(out))
}

/// Register the variant-filtering helpers shared by every backend.
/// Called from each per-language `register_*_filters` function so the same
/// filter names are available no matter which template engine is rendering.
pub fn register_invoke_filters(env: &mut minijinja::Environment) {
    env.add_filter("scxml", filter_scxml);
    env.add_filter("hybrid", filter_hybrid);
    env.add_filter("scxml_family", filter_scxml_family);
    env.add_filter("to_field_suffix", filter_to_field_suffix);
}

/// Split string on whitespace (replaces Python `.split()` method in templates).
fn filter_split(s: String) -> Vec<Value> {
    s.split_whitespace()
        .map(|w| Value::from(w.to_string()))
        .collect()
}

/// Return substring from index n (replaces Python `[n:]` slicing in templates).
/// Uses char-based indexing to avoid panics on multi-byte UTF-8 boundaries.
fn filter_slice_from(s: String, n: usize) -> String {
    s.chars().skip(n).collect()
}

/// Strip the SCXML auto-id leading underscore so an invoke id can be embedded
/// directly in a generated field/variable name without producing a double
/// underscore (`child_` + `_invoke_0` → `child__invoke_0`). Mirrors the
/// `field_suffix` derivation in [`crate::parser`] for sites where only the
/// raw id string is available (e.g., `#_<invokeId>` send targets).
fn filter_to_field_suffix(s: String) -> String {
    s.trim_start_matches('_').to_string()
}

// ── Go filters ──────────────────────────────────────────────────

/// Go reserved keywords — must be escaped with `_` suffix
const GO_KEYWORDS: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else",
    "fallthrough", "for", "func", "go", "goto", "if", "import", "interface",
    "map", "package", "range", "return", "select", "struct", "switch", "type",
    "var",
];

/// Register all Go-specific filters on the minijinja environment.
pub fn register_go_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_camel_case", to_camel_case);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_go_type", to_go_type);
    env.add_filter("escape_go", escape_go);
    env.add_filter("to_go_string_expr", to_go_string_expr);
    env.add_filter("to_event_variant", to_event_variant);
    env.add_filter("to_state_variant", to_state_variant);
    env.add_filter("to_go_literal", to_go_literal);
    env.add_filter("escape_go_keyword", escape_go_keyword);
    env.add_filter("to_in_predicate_go", to_in_predicate_go);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("to_machine_name", to_pascal_case);
    // ECMAScript→Lua transformation filters (shared with Rust)
    env.add_filter("to_lua_expr", to_lua_expr);
    env.add_filter("to_lua_guard", to_lua_guard);
    env.add_filter("to_lua_script", to_lua_script);
    // Cross-engine compatibility filters
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
}

/// Map SCXML variable type to Go type.
fn to_go_type(var_type: String) -> String {
    match var_type.as_str() {
        "int" => "int64".to_string(),
        "string" => "string".to_string(),
        "bool" => "bool".to_string(),
        _ => "sce.ScriptValue".to_string(),
    }
}

/// Escape characters for Go string literals.
fn escape_go(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Convert SCXML string expression to Go string expression.
fn to_go_string_expr(expr: String) -> String {
    if expr.is_empty() {
        return "\"\"".to_string();
    }
    let stripped = expr.trim();
    if stripped.len() >= 2 && stripped.starts_with('\'') && stripped.ends_with('\'') {
        let inner = &stripped[1..stripped.len() - 1];
        let inner = inner.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{inner}\"")
    } else {
        expr
    }
}

/// Render a value as a Go literal expression.
fn to_go_literal(value: Value) -> String {
    if value.is_none() || value.is_undefined() {
        return "nil".to_string();
    }
    if let Ok(b) = bool::try_from(value.clone()) {
        return if b { "true" } else { "false" }.to_string();
    }
    if let Ok(i) = i64::try_from(value.clone()) {
        return i.to_string();
    }
    if let Ok(f) = f64::try_from(value.clone()) {
        return format!("{f}");
    }
    if let Some(s) = value.as_str() {
        let escaped = escape_go(s.to_string());
        return format!("\"{escaped}\"");
    }
    "nil".to_string()
}

/// Suffix Go keywords with `_` to make them valid identifiers.
fn escape_go_keyword(name: String) -> String {
    if GO_KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// Transform C++ In() predicate code to Go.
fn to_in_predicate_go(cond_cpp: String) -> String {
    if cond_cpp.is_empty() {
        return String::new();
    }
    let result = cond_cpp.replace("this->", "");
    RE_IS_STATE_ACTIVE
        .replace_all(&result, r#"p.IsStateActive("$1")"#)
        .to_string()
}

// ── C++ filters ──────────────────────────────────────────────────

/// Register all C++-specific filters on the minijinja environment.
pub fn register_cpp_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("capitalize", capitalize_state);
    env.add_filter("escape_cpp", escape_cpp);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
}

/// Capitalize state/event names for C++ enums.
fn capitalize_state(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    match name.to_lowercase().as_str() {
        "pass" => "Pass".to_string(),
        "fail" => "Fail".to_string(),
        _ => capitalize_first(&name),
    }
}

/// Escape C++ string literals (identical escaping rules to Rust).
fn escape_cpp(text: String) -> String {
    escape_rust(text)
}

// ── Kotlin filters ───────────────────────────────────────────────

/// Register all Kotlin-specific filters on the minijinja environment.
pub fn register_kotlin_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_camel_case", to_camel_case);
    env.add_filter("to_kotlin_type", to_kotlin_type);
    env.add_filter("escape_kotlin", escape_kotlin);
    env.add_filter("to_kotlin_string_expr", to_kotlin_string_expr);
    env.add_filter("to_event_class_name", to_event_class_name);
    env.add_filter("to_state_class_name", to_state_class_name);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
}

/// Convert identifier to camelCase for Kotlin property names.
pub fn to_camel_case(name: String) -> String {
    if name.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = RE_DELIMITERS.split(&name).collect();
    let mut result = String::new();
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        if i == 0 {
            result.push_str(p);
        } else {
            result.push_str(&capitalize_first(p));
        }
    }
    result
}

/// Map SCXML variable type to Kotlin type.
fn to_kotlin_type(var_type: String) -> String {
    match var_type.as_str() {
        "int" => "Int".to_string(),
        "string" => "String".to_string(),
        "bool" => "Boolean".to_string(),
        _ => "Any".to_string(),
    }
}

/// Escape Kotlin string literals.
fn escape_kotlin(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('$', "\\$")
}

/// Convert SCXML string expression to Kotlin string expression.
fn to_kotlin_string_expr(expr: String) -> String {
    if expr.is_empty() {
        return "\"\"".to_string();
    }
    let stripped = expr.trim();
    if stripped.len() >= 2 && stripped.starts_with('\'') && stripped.ends_with('\'') {
        let inner = &stripped[1..stripped.len() - 1];
        let inner = inner.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$");
        format!("\"{inner}\"")
    } else {
        expr
    }
}

/// Convert dot-separated SCXML event name to Kotlin nested class reference.
pub fn to_event_class_name(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    name.split('.')
        .map(|dot_part| {
            RE_WORD_DELIMITERS
                .split(dot_part)
                .map(|p| if p.is_empty() { String::new() } else { capitalize_first(p) })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Convert SCXML state ID to Kotlin PascalCase class name.
/// Identical to to_state_variant — both split on underscore/hyphen only.
fn to_state_class_name(name: String) -> String {
    to_state_variant(name)
}
