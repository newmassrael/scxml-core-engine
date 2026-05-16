// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    env.add_filter("to_rust_variant", to_rust_variant);
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
    env.add_filter("extern_callback_path", filter_extern_callback_path);
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

/// Convert all-uppercase identifiers to PascalCase for Rust enum variants.
/// "STOP" -> "Stop", "RUNNING" -> "Running", "ENGINE_START" -> "EngineStart".
/// Mixed-case input is delegated to to_pascal_case.
pub fn to_rust_variant(name: String) -> String {
    if name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
        name.split('_')
            .filter(|p| !p.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => {
                        let mut out = c.to_uppercase().to_string();
                        out.extend(chars.map(|c| c.to_ascii_lowercase()));
                        out
                    }
                    None => String::new(),
                }
            })
            .collect()
    } else {
        to_pascal_case(name)
    }
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

/// W3C SCXML 5.2.2: read external data file referenced by `<data src="...">`.
///
/// C11 codegen-time read (RFC §5.J.1 R3 zero-deps lock-in: no runtime
/// fopen in sce-c-runtime). Mirrors cpp `FileLoadingHelper::loadExternalScript`
/// + `DataModelInitHelper::initializeVariableFromSrc` by inlining the file
/// content into the generated C source as a string literal. The downstream
/// emit threads this content through the same eval+whitespace-fallback
/// dispatch as inline `<data>` content (cpp `DataModelInitHelper::initializeVariable`
/// at sce/src/common/DataModelInitHelper.cpp:80-156).
///
/// `base_path` is `model.scxml_base_path` (relative to codegen cwd or the
/// canonical SCXML parent when the strip fails). cmake's c11 fixture path
/// copies the `.txt` sidecar alongside the SCXML so this resolves at the
/// build location; manual sce-codegen invocation reads from the original
/// resources/<n>/ directory because the SCXML path is absolute and
/// compute_scxml_base_path returns the absolute parent when cwd-strip fails.
fn read_data_src(src: String, base_path: String) -> Result<String, minijinja::Error> {
    let path = src.strip_prefix("file:").unwrap_or(&src);
    let candidate = std::path::PathBuf::from(&base_path).join(path);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        std::env::current_dir()
            .map(|c| c.join(&candidate))
            .unwrap_or(candidate)
    };
    std::fs::read_to_string(&resolved).map_err(|e| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!(
                "read_data_src failed for src='{src}' (resolved='{}'): {e}",
                resolved.display()
            ),
        )
    })
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

/// Is the invoke a remote mesh `<invoke type="scxml" src="#peer">`?
/// True when `remote_mesh_target` is set — i.e. the classifier in
/// [`crate::inject_partition_context_for`] matched `src` to a distinct
/// mesh machine declared in `deploy.yaml`. Used by `filter_scxml` and
/// `filter_scxml_family` to exclude remote invokes from W3C local-session
/// machinery (child session field, pending-invoke queue, finalize
/// helper); `filter_scxml_remote` is the positive form used by the
/// §10.7.1 `SESSION_F_NOT_IMPLEMENTED` raise site.
fn invoke_is_remote_mesh(item: &Value) -> bool {
    match item.get_attr("remote_mesh_target") {
        Ok(v) => !v.is_none() && !v.is_undefined(),
        Err(_) => false,
    }
}

/// Keep only **local** `Invoke::Scxml` entries — remote mesh
/// `<invoke type="scxml" src="#peer">` (SCE_MESH.md §9.6) are not W3C
/// local-session invokes and are routed to `filter_scxml_remote`.
fn filter_scxml(value: Value) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        let Ok(kind_val) = item.get_attr("kind") else { continue };
        let Some(k) = kind_val.as_str() else { continue };
        if k != "Scxml" {
            continue;
        }
        if invoke_is_remote_mesh(&item) {
            continue;
        }
        out.push(item.clone());
    }
    Ok(Value::from(out))
}

/// Keep only **remote mesh** `Invoke::Scxml` entries — i.e. `src="#peer"`
/// referencing a distinct mesh machine per SCE_MESH.md §9.6. Consumed
/// by the C++ entry-action template to emit the §10.7.1
/// `SESSION_F_NOT_IMPLEMENTED` raise until Session F lands the wire
/// patterns 14-20 runtime.
fn filter_scxml_remote(value: Value) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        let Ok(kind_val) = item.get_attr("kind") else { continue };
        let Some(k) = kind_val.as_str() else { continue };
        if k != "Scxml" {
            continue;
        }
        if !invoke_is_remote_mesh(&item) {
            continue;
        }
        out.push(item.clone());
    }
    Ok(Value::from(out))
}

/// Keep only `Invoke::Hybrid` entries.
fn filter_hybrid(value: Value) -> Result<Value, minijinja::Error> {
    filter_invokes_by_kind(value, "Hybrid")
}

/// Keep local `Invoke::Scxml` and `Invoke::Hybrid` — the W3C
/// SCXML-session kinds. Remote mesh invokes are excluded for the same
/// reason they are excluded from `filter_scxml`: they are not W3C
/// local-session invokes and have no local child-session machinery.
fn filter_scxml_family(value: Value) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        if let Ok(kind_val) = item.get_attr("kind") {
            if let Some(k) = kind_val.as_str() {
                if k == "Hybrid" {
                    out.push(item.clone());
                } else if k == "Scxml" && !invoke_is_remote_mesh(&item) {
                    out.push(item.clone());
                }
            }
        }
    }
    Ok(Value::from(out))
}

/// Keep only `Invoke::MeshRpc` entries — the SCE_MESH.md §9.5 short-lived RPC
/// invoke kind. Consumed by state templates that emit onentry/onexit hooks
/// around the generated `TransportRouter::invoke_<suffix>` / `cancel_<suffix>`
/// methods; paired with the topology-side `ResolvedTarget.invoke_sites` that
/// drives those method emissions.
fn filter_mesh_rpc(value: Value) -> Result<Value, minijinja::Error> {
    filter_invokes_by_kind(value, "MeshRpc")
}

/// Register the variant-filtering helpers shared by every backend.
/// Called from each per-language `register_*_filters` function so the same
/// filter names are available no matter which template engine is rendering.
pub fn register_invoke_filters(env: &mut minijinja::Environment) {
    env.add_filter("scxml", filter_scxml);
    env.add_filter("scxml_remote", filter_scxml_remote);
    env.add_filter("hybrid", filter_hybrid);
    env.add_filter("scxml_family", filter_scxml_family);
    env.add_filter("mesh_rpc", filter_mesh_rpc);
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

/// watching-zenoh RFC §5.E B7-η' Atomic A2 + W1.4 — strip the
/// language prefix from `<sce:on-sample callback="...">` to produce
/// the bare path the codegen emits at the call site. Today only
/// `rust:` (Q-Callback-2 v1) is valid; the validator
/// (`validate_on_sample_callback_paths`) rejects every other
/// shape before this filter ever runs, so the prefix is always
/// `rust:` here. Future language axes (`c:`, `kotlin:`, …) extend
/// the same filter via the same pattern — they will arrive with
/// their own per-backend dispatch sites and won't share this one.
fn filter_extern_callback_path(s: String) -> String {
    if let Some(rest) = s.strip_prefix("rust:") {
        return rest.to_string();
    }
    s
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
    env.add_filter("extern_callback_path", filter_extern_callback_path);
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
    env.add_filter("extern_callback_path", filter_extern_callback_path);
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

// ── C11 filters ─────────────────────────────────────────────────

/// Register all C11-specific filters on the minijinja environment.
/// RFC §5.J.1 (watching-zenoh consumer / MCU AOT backend).
///
/// `escape_c` shares the C++/Rust escape rule set (backslash, double
/// quote, newline, carriage return, tab); C11 string literals follow
/// the same escape grammar so we can route the filter to `escape_rust`
/// with no behavioural change. The Lua expression family is registered
/// here (not just on Rust/Go) because the C11 backend transpiles
/// ECMAScript expressions to Lua at codegen time and embeds the result
/// as a C string literal passed through `luaL_dostring`.
pub fn register_c11_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("escape_c", escape_c);
    env.add_filter("escape_json_string", escape_json_string);
    env.add_filter("to_lua_expr", to_lua_expr);
    env.add_filter("to_lua_guard", to_lua_guard);
    env.add_filter("to_lua_script", to_lua_script);
    env.add_filter("to_in_predicate_c11", to_in_predicate_c11);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("read_data_src", read_data_src);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    // watching-zenoh RFC §5.E B7-η' W2: per-link function names
    // (`<machine>_deliver_link_<X>_sample`) snake-case the link
    // name to keep the C identifier stable when the SCXML link
    // attribute uses kebab-case or mixedCase.
    env.add_filter("to_snake_case", to_snake_case);
}

/// Escape C string literals (identical escaping rules to Rust/C++).
fn escape_c(text: String) -> String {
    escape_rust(text)
}

/// Escape characters for embedding inside a JSON string literal (RFC 8259 §7).
/// SSoT mirror of cpp `DoneDataHelper::escapeJsonString`
/// (sce/include/common/DoneDataHelper.h:260-291). Only the inner escapes
/// are produced — surrounding `"..."` quotes are added by the template.
/// Composed with `escape_c` when emitted into a C string literal so the
/// runtime bytes match cpp's `EventDataHelper::scriptValueToJsonString`
/// for a string ScriptValue (`"text"` JSON-quoted form, mesh §9.6.2
/// wire-18 canonical wire). The replace order is fixed: backslash first
/// so the `\` introduced by subsequent escapes is not double-escaped.
pub fn escape_json_string(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\u{08}', "\\b")
        .replace('\u{0c}', "\\f")
}

/// W3C SCXML 5.9.2: rewrite pure In('xxx') predicate text to a C11 native
/// `<machine>_in_state(sm, <MACHINE>_STATE_<XXX>)` call. Mirrors cpp
/// `parser::convert_in_to_cpp` which substitutes `this->isStateActive("xxx")`
/// — both sit at the codegen-time-text-substitution layer (T3 inline-only
/// lock-in for C11) instead of routing through a runtime `In()` callback.
/// The state-id transformation matches `state_machine.h.jinja2`'s enum
/// emission (uppercase + dot/dash → underscore) so the produced symbol
/// resolves against the per-fixture `<machine>_state_e` enum.
///
/// W3C 3.10 carve-out: history pseudo-states are *never* in the active
/// configuration, so `In('history_id')` must always return false (cpp
/// gets this for free because `isStateActive` looks up activeStates_ as
/// a string set; C11 must emit a `false` literal because history IDs
/// have a separate `<machine>_history_e` enum and would not resolve as
/// `<MACHINE>_STATE_*`). The `history_ids` arg carries the per-model
/// history-state-id list from `model.history_states` so the filter can
/// branch at codegen time.
fn to_in_predicate_c11(
    cond_text: String,
    machine_name: String,
    history_ids: Vec<String>,
) -> String {
    if cond_text.is_empty() {
        return String::new();
    }
    static RE_IN_PRED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"In\(['"]([^'"]+)['"]\)"#).unwrap());
    let upper_machine = machine_name.to_uppercase();
    RE_IN_PRED
        .replace_all(&cond_text, |caps: &regex::Captures| {
            let state_id = &caps[1];
            if history_ids.iter().any(|h| h == state_id) {
                return "false".to_string();
            }
            let upper_state = state_id.to_uppercase().replace(['-', '.'], "_");
            format!("{machine_name}_in_state(sm, {upper_machine}_STATE_{upper_state})")
        })
        .to_string()
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
    env.add_filter("extern_callback_path", filter_extern_callback_path);
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

// ── Python filters ──────────────────────────────────────────────

/// Register all Python-specific filters on the minijinja environment.
///
/// Python AOT (Atomic α): state/event variants are `IntEnum` members named
/// in UPPER_SNAKE_CASE (`State.IDLE`, `Event.TEMP_HIGH`). Identifiers from
/// SCXML are normalised via `to_python_const`; script bodies are emitted
/// as repr-quoted Python string literals via `py_string_literal` so the
/// template never has to worry about embedded quotes / newlines / unicode.
/// A `PYTHON_KEYWORDS` / `escape_python_keyword` pair is intentionally
/// absent from Atomic α — every emitted identifier is UPPER_SNAKE_CASE
/// (IntEnum member) so keyword collisions cannot occur at this layer.
/// Atomic β will introduce datamodel variable names where keyword escape
/// becomes load-bearing; the pair lands then alongside its first consumer
/// to keep the cross-backend "filter + consumer atomic" rule
/// ([[feedback-built-but-unconsumed]]).
pub fn register_python_filters(env: &mut minijinja::Environment) {
    register_invoke_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_python_const", to_python_const);
    env.add_filter("py_string_literal", py_string_literal);
    // ECMAScript→Lua transformer filters — Python AOT now joins the
    // Rust / Go / Kotlin / C11 / C++ family that emits Lua text and
    // evaluates it through the IScriptEngine layer. The filters are
    // identical to those backends; templates pipe author expressions
    // through `| to_lua_expr | py_string_literal` so the generated
    // module hands Lua text (not Python) to the script engine.
    env.add_filter("to_lua_expr", to_lua_expr);
    env.add_filter("to_lua_guard", to_lua_guard);
    env.add_filter("to_lua_script", to_lua_script);
    env.add_filter("to_event_variant", to_event_variant);
    env.add_filter("to_state_variant", to_state_variant);
    env.add_filter("to_machine_name", to_pascal_case);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    // W3C SCXML 5.2.2: `<data src="file:...">` inlining at codegen time —
    // same filter the C11 backend uses (see
    // `tools/codegen/templates/c/scriptengine.jinja2:272`) so Python
    // can route loaded text through `_init_data_with_content` instead
    // of a runtime `fopen`.
    env.add_filter("read_data_src", read_data_src);
}

/// Convert an SCXML identifier (state id / event name) into a Python `IntEnum`
/// member name. The result is UPPER_SNAKE_CASE with `.` / `-` mapped to `_`.
///
/// Example: `"temp_high"` → `"TEMP_HIGH"`, `"error.execution"` → `"ERROR_EXECUTION"`,
/// `"passingState"` → `"PASSING_STATE"`. Empty input maps to `"EMPTY"`.
/// Names that collide with Python soft-keywords (`match`, `case`) or that
/// happen to start with a digit get an underscore prefix so the resulting
/// member is a valid identifier.
pub fn to_python_const(name: String) -> String {
    if name.is_empty() {
        return "EMPTY".to_string();
    }
    let name = name.replace(['.', '-'], "_");
    let chars: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len() + 4);
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_ascii_uppercase() {
            let prev = chars[i - 1];
            if prev.is_ascii_lowercase() || prev.is_ascii_digit() {
                result.push('_');
            }
        }
        result.push(ch);
    }
    let upper = result.to_uppercase();
    let prefixed = if upper.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("_{upper}")
    } else {
        upper
    };
    // Avoid collisions with the manually-injected `NULL` sentinel.
    if prefixed == "NULL" {
        return "NULL_".to_string();
    }
    prefixed
}

/// Render a string as a Python source literal. Uses double-quoted form with
/// backslash escapes — `repr` would also work but always picks single quotes
/// for unicode-free strings, which clashes with the rest of the generated
/// source's double-quote convention.
pub fn py_string_literal(text: String) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

