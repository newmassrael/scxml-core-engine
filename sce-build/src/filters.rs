// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// Jinja2 template filters — multi-language filter registry.
// Rust, C++, and Kotlin filters registered with minijinja for template rendering.

use minijinja::Value;
use regex::Regex;

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
    let re = Regex::new(r"[._\-]").unwrap();
    let parts: Vec<&str> = re.split(&name).collect();
    parts
        .iter()
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                let mut chars = p.chars();
                let first = chars.next().unwrap().to_uppercase().to_string();
                first + chars.as_str()
            }
        })
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
pub fn to_event_variant(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    let re = Regex::new(r"[._\-]").unwrap();
    let parts: Vec<&str> = re.split(&name).collect();
    parts
        .iter()
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                let mut chars = p.chars();
                let first = chars.next().unwrap().to_uppercase().to_string();
                first + chars.as_str()
            }
        })
        .collect()
}

/// Convert SCXML state ID to Rust enum variant PascalCase.
pub fn to_state_variant(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    // State variants only split on _ and -, NOT on . (unlike event variants)
    let re = Regex::new(r"[_\-]").unwrap();
    let parts: Vec<&str> = re.split(&name).collect();
    parts
        .iter()
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                let mut chars = p.chars();
                let first = chars.next().unwrap().to_uppercase().to_string();
                first + chars.as_str()
            }
        })
        .collect()
}

/// Render a value as a Rust literal expression.
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
    let re = Regex::new(r#"isStateActive\("([^"]+)"\)"#).unwrap();
    re.replace_all(&result, r#"self.is_state_active("$1")"#)
        .to_string()
}

/// W3C SCXML B.2: Normalize whitespace.
fn normalize_ws(text: String) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text.trim(), " ").to_string()
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

/// Split string on whitespace (replaces Python `.split()` method in templates).
fn filter_split(s: String) -> Vec<Value> {
    s.split_whitespace()
        .map(|w| Value::from(w.to_string()))
        .collect()
}

/// Return substring from index n (replaces Python `[n:]` slicing in templates).
fn filter_slice_from(s: String, n: usize) -> String {
    if n >= s.len() {
        String::new()
    } else {
        s[n..].to_string()
    }
}

// ── C++ filters ──────────────────────────────────────────────────

/// Register all C++-specific filters on the minijinja environment.
pub fn register_cpp_filters(env: &mut minijinja::Environment) {
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
        _ => {
            let mut chars = name.chars();
            let first = chars.next().unwrap().to_uppercase().to_string();
            first + chars.as_str()
        }
    }
}

/// Escape C++ string literals.
fn escape_cpp(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ── Kotlin filters ───────────────────────────────────────────────

/// Register all Kotlin-specific filters on the minijinja environment.
pub fn register_kotlin_filters(env: &mut minijinja::Environment) {
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
fn to_camel_case(name: String) -> String {
    if name.is_empty() {
        return String::new();
    }
    let re = Regex::new(r"[._\-]").unwrap();
    let parts: Vec<&str> = re.split(&name).collect();
    let mut result = String::new();
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        if i == 0 {
            result.push_str(p);
        } else {
            let mut chars = p.chars();
            let first = chars.next().unwrap().to_uppercase().to_string();
            result.push_str(&first);
            result.push_str(chars.as_str());
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
    let dot_parts: Vec<&str> = name.split('.').collect();
    let result: Vec<String> = dot_parts
        .iter()
        .map(|dot_part| {
            let re = Regex::new(r"[_\-]").unwrap();
            let sub_parts: Vec<&str> = re.split(dot_part).collect();
            sub_parts
                .iter()
                .map(|p| {
                    if p.is_empty() {
                        String::new()
                    } else {
                        let mut chars = p.chars();
                        let first = chars.next().unwrap().to_uppercase().to_string();
                        first + chars.as_str()
                    }
                })
                .collect::<String>()
        })
        .collect();
    result.join(".")
}

/// Convert SCXML state ID to Kotlin PascalCase class name.
fn to_state_class_name(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    let re = Regex::new(r"[_\-]").unwrap();
    let parts: Vec<&str> = re.split(&name).collect();
    parts
        .iter()
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                let mut chars = p.chars();
                let first = chars.next().unwrap().to_uppercase().to_string();
                first + chars.as_str()
            }
        })
        .collect()
}
