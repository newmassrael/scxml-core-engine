// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Jinja2 template filters — multi-language filter registry.
// Rust, C++, and Kotlin filters registered with minijinja for template rendering.

use minijinja::Value;
use regex::Regex;
use std::sync::{Arc, LazyLock};

use crate::ecmascript::DocumentScope;

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

/// The value the SCXML specification binds to `_name`.
///
/// The spec is explicit about both halves: the processor binds `_name`
/// "at load time to the value of the `name` attribute of the `<scxml>`
/// element", and generates a name when the document declares none. The
/// section is cited in the body rather than here — a doc comment does
/// not resolve to its own symbol, so a citation placed here binds
/// nothing and reports `binding_unbacked`. `declared` is the attribute
/// (`SCXMLModel::scxml_name`, empty when absent) and `generated` is the
/// identity the toolchain derives from the document itself
/// (`SCXMLModel::name`, the file stem).
///
/// This exists as a filter rather than as prose in five templates
/// because five templates is how it went wrong: every AOT backend
/// interpolated the *generated* identity into the engine's
/// `setupSystemVariables` call, so a document declaring
/// `name="machineName"` bound `_name` to its file stem. The C11 backend
/// alone read the attribute, which is what showed the others were not
/// expressing a different policy — they were simply wrong.
///
/// Callers must still apply their language's escape filter: unlike the
/// file stem this replaced, the value is author-controlled text that
/// lands inside a host string literal.
pub fn w3c_session_name(declared: &str, generated: &str) -> String {
    // §scxml-5.10: `_name` is bound at load time to the value of the
    // `name` attribute of the `<scxml>` element; §3.2.1 requires the
    // processor to generate one when the attribute is absent, and the
    // document identity is that generated name.
    if declared.is_empty() {
        generated.to_string()
    } else {
        declared.to_string()
    }
}

/// Rust 2021 edition reserved keywords — must be escaped with `r#` prefix
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", // Reserved for future use
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized",
    "virtual", "yield", "try", "union",
];

/// Register all Rust-specific filters on the minijinja environment.
pub fn register_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_upper_snake_case", to_upper_snake_case);
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
    register_ecmascript_filters(env, scope);
    // Cross-engine compatibility filters (replace Python-specific string methods)
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
}

/// The ECMAScript→Lua seam, in the four backends that run the datamodel
/// on a Lua interpreter.
///
/// One function rather than four identical blocks because the seam has a
/// contract — see [`to_lua_expr`] — and the document scope is part of it.
/// A backend that registered three of the four filters, or registered
/// them against a scope of its own, would answer a different question
/// about the same document than `check` does.
fn register_ecmascript_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    let for_expr = Arc::clone(scope);
    env.add_filter("to_lua_expr", move |expr: String| {
        to_lua_expr(expr, &for_expr)
    });
    let for_content = Arc::clone(scope);
    env.add_filter("to_lua_data_content", move |content: String| {
        to_lua_data_content(content, &for_content)
    });
    let for_guard = Arc::clone(scope);
    env.add_filter("to_lua_guard", move |expr: String| {
        to_lua_guard(expr, &for_guard)
    });
    let for_script = Arc::clone(scope);
    env.add_filter("to_lua_script", move |script: String| {
        to_lua_script(script, &for_script)
    });
    // No scope: a write is how this datamodel's globals come into
    // existence, so the target is not resolved against what the document
    // declares. See [`crate::ecmascript::to_lua_location`].
    env.add_filter("to_lua_location", to_lua_location);
    env.add_filter("escape_lua", escape_lua);
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

/// Convert identifier to UPPER_SNAKE_CASE for macros, constant names, and
/// `_MAX_BYTES`-style enclosing-scope references in generated code.
pub fn to_upper_snake_case(name: String) -> String {
    to_snake_case(name).to_uppercase()
}

/// Convert identifier to PascalCase for Rust struct/enum/variant names.
pub fn to_pascal_case(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_string();
    }
    RE_DELIMITERS
        .split(&name)
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                capitalize_first(p)
            }
        })
        .collect()
}

/// Convert all-uppercase identifiers to PascalCase for Rust enum variants.
/// "STOP" -> "Stop", "RUNNING" -> "Running", "ENGINE_START" -> "EngineStart".
/// Mixed-case input is delegated to to_pascal_case.
pub fn to_rust_variant(name: String) -> String {
    if name
        .chars()
        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
    {
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
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                capitalize_first(p)
            }
        })
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

/// §scxml-B-2: Normalize whitespace.
fn normalize_ws(text: String) -> String {
    RE_WHITESPACE.replace_all(text.trim(), " ").to_string()
}

/// §scxml-5.2.2: read external data file referenced by `<data src="...">`.
///
/// C11 codegen-time read (RFC §synth-5-J-1 zero-deps rule: no runtime
/// fopen in sce-c-runtime). Mirrors cpp `FileLoadingHelper::loadExternalScript` +
/// `DataModelInitHelper::initializeVariableFromSrc` by inlining the file
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

/// The three seams between a template and [`crate::ecmascript`].
///
/// # What a rejection becomes
///
/// Source the frontend cannot parse does **not** stop code generation. W3C
/// SCXML §5.9.1 says an unevaluable `cond` raises `error.execution` and
/// reads as false, and §5.4 says the same of an `<assign expr>`; test 344
/// (`cond="return"`, a reserved word) is in the conformance suite precisely
/// to check it. Refusing at codegen would make that document ungeneratable
/// rather than conformant.
///
/// So a rejection is emitted as Lua that raises when evaluated, carrying the
/// parser's message. That is the same outcome the transformer this replaced
/// reached — its output for `return` was `_scxml_truthy(return)`, which Lua
/// refuses to compile — except by design rather than by accident, and with a
/// message naming the expression instead of a Lua parse position.
///
/// The message travels in the raised error, which is where a reader of the
/// generated source finds it. It is deliberately *not* written to stderr:
/// under `--error-format=json` every stderr line is one diagnostic object
/// (`SCE_ERROR_CONTRACT.md` §4), and a bare warning line there is a
/// malformed record — `diagnostic_corpus_schema` fails on it. Surfacing a
/// refusal as a real `DiagnosticCode` is the follow-up that belongs to the
/// error-contract checklist, not to this seam.
pub fn to_lua_expr(expr: String, scope: &DocumentScope) -> Result<String, minijinja::Error> {
    // §scxml-B-2: a value expression — `<data expr>`, `<assign expr>`,
    // `<param expr>`, `<log expr>` — evaluated for what it yields.
    if expr.is_empty() {
        return Ok(String::new());
    }
    Ok(match crate::ecmascript::to_lua_value(&expr, scope) {
        Ok(lua) => lua,
        Err(err) => lua_that_raises("expr", &expr, err),
    })
}

fn to_lua_guard(expr: String, scope: &DocumentScope) -> Result<String, minijinja::Error> {
    // §scxml-B-2: a condition — `transition/@cond`, `<if>`, `<elseif>` —
    // evaluated under ECMAScript truthiness, which counts `0` and `""` as
    // false where Lua counts them as true.
    if expr.is_empty() {
        return Ok("true".to_string());
    }
    Ok(match crate::ecmascript::to_lua_condition(&expr, scope) {
        Ok(lua) => lua,
        Err(err) => lua_that_raises("cond", &expr, err),
    })
}

fn to_lua_script(script: String, scope: &DocumentScope) -> Result<String, minijinja::Error> {
    // §scxml-B-2: a `<script>` body — the one place the datamodel admits
    // statements rather than a single expression.
    if script.is_empty() {
        return Ok(String::new());
    }
    Ok(match crate::ecmascript::to_lua_script(&script, scope) {
        Ok(lua) => lua,
        Err(err) => lua_that_raises("script", &script, err),
    })
}

/// An authored write target — `<assign location>`, `<send idlocation>`,
/// `<foreach item>`, `<foreach index>` — lowered to the Lua the engine
/// underneath actually assigns to.
///
/// The same seam the reads go through, for the same reason: ECMA-262
/// 11.2.1 defines `arr[0]` once, and a document that writes it through
/// one lowering and reads it through another names two cells. Splicing
/// the author's text is what every template did, so `<assign
/// location="arr[0]"/>` wrote a Lua table's zeroth slot while every read
/// of `arr[0]` measured its first.
pub fn to_lua_location(location: String) -> Result<String, minijinja::Error> {
    if location.is_empty() {
        return Ok(String::new());
    }
    Ok(match crate::ecmascript::to_lua_location(&location) {
        Ok(lua) => lua,
        Err(err) => lua_target_that_raises(&location, err),
    })
}

/// A Lua *assignment target* that raises the parser's message instead of
/// naming a cell.
///
/// A refused target has to survive as far as the assignment, which means
/// it has to stay an assignment target. `error("…") = 1` is not one:
/// Lua's grammar admits a function call as a statement, never as the
/// left side of `=`, so the chunk would fail to *parse* and the author's
/// message would be replaced by a syntax error at the one moment it is
/// needed. Indexing the raise keeps the statement grammatical — Lua 5.4
/// §3.5 makes `'(' exp ')' '[' exp ']'` a var — and the prefix is
/// evaluated before any assignment happens, so nothing is written.
fn lua_target_that_raises(source: &str, err: crate::ecmascript::ExprError) -> String {
    // §scxml-5.4: a location that denotes nothing is an `error.execution`
    // at run time rather than a document that cannot be generated, so the
    // codegen-time verdict is carried to the runtime the clause describes
    // instead of refusing the document.
    format!("({})[1]", lua_that_raises("location", source, err))
}

/// Lua that raises the parser's message when the engine evaluates it.
fn lua_that_raises(what: &str, source: &str, err: crate::ecmascript::ExprError) -> String {
    // §scxml-5.9.1: a condition the processor cannot evaluate is treated as
    // false, with `error.execution` placed on the internal queue. Every
    // backend's guard path already turns an evaluation error into exactly
    // that, so raising is how a codegen-time verdict reaches the runtime the
    // clause describes.
    let message = format!("SCXML {what} is not valid ECMAScript: {source}: {err}");
    format!(
        "error({})",
        crate::ecmascript::lua::string_literal(&message)
    )
}

/// Inline `<data>` / `<content>` text, lowered to Lua.
///
/// §scxml-B-2 makes this the one place where "not an expression" is an
/// answer rather than an error: content that is neither XML nor JSON *is a
/// string*, whitespace-normalized. Every other seam raises on what it
/// cannot parse; this one chooses among the readings the spec gives it —
/// and hands XML on untouched, because building a DOM node is the
/// initializer's job, not this filter's.
///
/// Deciding here rather than at runtime is the change. The generated code
/// used to hand the engine whatever the string rewriter produced and let
/// evaluation *fail* to discover that the text was not an expression — so
/// `<data>this  is \na string</data>` reached Lua as three bare identifiers,
/// and the string only appeared because the parse error triggered a
/// fallback. The reading is decidable without running anything, so it is
/// made where it can be seen.
pub fn to_lua_data_content(
    content: String,
    scope: &DocumentScope,
) -> Result<String, minijinja::Error> {
    if content.is_empty() {
        return Ok(String::new());
    }
    // The scope reaches here too, and it changes an answer: `<data>Date
    // </data>` is a *string*, not a reach for a global SCE lacks. Reading
    // it as an expression would have bound the name; the refusal sends it
    // to the string branch below, which is the reading §scxml-B-2 wants.
    if let Ok(lua) = crate::ecmascript::to_lua_value(&content, scope) {
        return Ok(lua);
    }
    // XML is the reading this filter must *not* make. §scxml-B-2 orders the
    // three: XML becomes a DOM value, JSON an object, anything else a
    // string — and only the initializer at the other end of this seam can
    // build a DOM node. Handing the source text back is what lets its XML
    // branch see the `<`; deciding "string" here would bind test557's
    // `<books>` as text and make `var1:getElementsByTagName(…)` fail on a
    // value that is no longer a document.
    if content.trim_start().starts_with('<') {
        return Ok(content);
    }
    Ok(lua_string_literal(&normalize_ws(content)))
}

/// Inline `<content>` text, kept in the author's own language.
///
/// The same reading as [`to_lua_data_content`], for the two backends that
/// hand the author's ECMAScript to an ECMAScript engine instead of
/// lowering it. Only the *answer* is language-specific: whether the text
/// is an expression at all is a question about the document, and asking
/// it differently per backend is how `<content>21</content>` could come
/// to mean a number in one generated machine and a string in another.
///
/// Accepted text is handed back untouched — it is already in the language
/// the engine speaks. Text that is not an expression becomes an
/// ECMAScript string literal, which is §scxml-B-2's second reading, and
/// XML is passed through for the same reason [`to_lua_data_content`]
/// passes it through: the DOM node is built at the other end of the seam.
pub fn to_author_data_content(
    content: String,
    scope: &DocumentScope,
) -> Result<String, minijinja::Error> {
    if content.is_empty() {
        return Ok(String::new());
    }
    if crate::ecmascript::to_lua_value(&content, scope).is_ok() {
        return Ok(content);
    }
    if content.trim_start().starts_with('<') {
        return Ok(content);
    }
    Ok(ecmascript_string_literal(&normalize_ws(content)))
}

/// Render text as an ECMAScript string literal.
///
/// Single-quoted because that is the spelling the conformance suite's own
/// documents use (`<content>'foo'</content>`), and because the result is
/// embedded in a C++ or Kotlin double-quoted literal by the escaper at
/// the next step — a quote that does not have to be escaped twice is one
/// fewer place the two escapings can disagree.
fn ecmascript_string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

/// Render text as a Lua string literal. Shares the emitter's escaping rules
/// by going through it, so a quote or a newline in `<data>` content cannot
/// be escaped one way here and another way there.
fn lua_string_literal(text: &str) -> String {
    crate::ecmascript::lua::string_literal(text)
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
        // Non-Invoke items simply fall through — filter returns only matches.
        if let Ok(kind_val) = item.get_attr("kind") {
            if let Some(k) = kind_val.as_str() {
                if k == want_kind {
                    out.push(item.clone());
                }
            }
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
/// §9.6.2 wire-14 dispatch site.
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
        let Ok(kind_val) = item.get_attr("kind") else {
            continue;
        };
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
/// by the C++ entry-action template to emit the §9.6.2 wire-14
/// `InvokeStart` dispatch, which falls back to the §10.7.1
/// `SESSION_F_TRANSPORT_UNAVAILABLE` raise when no transport binds the peer.
fn filter_scxml_remote(value: Value) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        let Ok(kind_val) = item.get_attr("kind") else {
            continue;
        };
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
                if k == "Hybrid" || (k == "Scxml" && !invoke_is_remote_mesh(&item)) {
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

/// Keep only `Invoke::Unsupported` entries — §scxml-6.4.1 `<invoke>` whose
/// `type` names no processor SCE implements. Consumed by the per-backend
/// entry-action and pending-invoke templates, which lower it to a single
/// `error.execution` raise at invoke time and nothing else.
fn filter_unsupported(value: Value) -> Result<Value, minijinja::Error> {
    filter_invokes_by_kind(value, "Unsupported")
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
    env.add_filter("unsupported", filter_unsupported);
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

/// SCE Protocol-Synthesis RFC §synth-5-E sample-callback lowering — strip the
/// language prefix from `<sce:on-sample callback="...">` to produce
/// the bare path the codegen emits at the call site. The validator
/// (`validate_on_sample_callback_paths`) rejects every prefix outside
/// [`CALLBACK_LANGUAGE_PREFIXES`] before this filter ever runs, so an
/// unstripped return here means a new prefix reached the validator
/// without reaching this list.
fn filter_extern_callback_path(s: String) -> String {
    for (_, prefix) in CALLBACK_LANGUAGE_PREFIXES {
        if let Some(rest) = s.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    s
}

/// The `<sce:on-sample callback>` language axes, as
/// `(language, prefix)`. One entry per backend that has a lowering for
/// the callback: Rust calls a module path, C11 calls a flat identifier
/// matching `sce_sub_callback_t`.
///
/// The single place the prefix strings live. Templates ask
/// [`filter_callback_lang`] which axis a callback is on rather than
/// testing for `"rust:"` themselves, so a backend cannot start
/// emitting another backend's callback by copying a string literal.
const CALLBACK_LANGUAGE_PREFIXES: &[(&str, &str)] = &[("rust", "rust:"), ("c", "c:")];

/// SCE Protocol-Synthesis RFC §synth-5-E — name the language axis of an
/// `<sce:on-sample callback="...">` value (`"rust"` / `"c"`).
///
/// Each backend emits a call only for its own axis: a `rust:` path has
/// no C lowering and a `c:` identifier does not resolve in Rust, so the
/// backend that does not own the axis skips the call and emits the
/// event raise alone. Returns the empty string for a value carrying no
/// known prefix, which the validator has already made unreachable —
/// the empty string then matches no template arm, which is the safe
/// direction.
fn filter_callback_lang(s: String) -> String {
    for (lang, prefix) in CALLBACK_LANGUAGE_PREFIXES {
        if s.starts_with(prefix) {
            return (*lang).to_string();
        }
    }
    String::new()
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
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// Register all Go-specific filters on the minijinja environment.
pub fn register_go_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    env.add_filter("w3c_session_name", w3c_session_name);
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
    register_ecmascript_filters(env, scope);
    // Cross-engine compatibility filters
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
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
///
/// The scope is here for one filter. This backend does not lower the
/// author's ECMAScript — it hands it to an ECMAScript engine — so it has
/// no use for the seam [`register_ecmascript_filters`] installs. It does
/// need the one question that seam can answer: §scxml-B-2 makes "is this
/// inline `<content>` an expression at all?" decide the *value*, and a
/// backend answering it for itself is how the same document comes to
/// carry a number in one generated machine and a string in another.
pub fn register_cpp_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    let for_content = Arc::clone(scope);
    env.add_filter("to_author_data_content", move |content: String| {
        to_author_data_content(content, &for_content)
    });
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    env.add_filter("capitalize", capitalize_state);
    env.add_filter("escape_cpp", escape_cpp);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
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
/// RFC §synth-5-J-1 (downstream consumer / MCU AOT backend).
///
/// `escape_c` shares the C++/Rust escape rule set (backslash, double
/// quote, newline, carriage return, tab); C11 string literals follow
/// the same escape grammar so we can route the filter to `escape_rust`
/// with no behavioural change. The Lua expression family is registered
/// here (not just on Rust/Go) because the C11 backend transpiles
/// ECMAScript expressions to Lua at codegen time and embeds the result
/// as a C string literal passed through `luaL_dostring`.
pub fn register_c11_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    // §scxml-B-2: the C11 backend has no file to load at runtime — RFC
    // §synth-5-J-1 forbids a runtime `fopen` in `sce-c-runtime` — so the
    // shared semantics travel into the generated source as a C string
    // literal. Same bytes as every other engine loads; `include_str!` binds
    // it at compile time so there is no path to resolve and no copy to keep
    // in step.
    env.add_global(
        "ecma_semantics_lua_c",
        c_lua_dostring_chunks(ECMA_SEMANTICS_LUA),
    );
    // §scxml-B-2 requires `JSON` in an ECMAScript datamodel, and C11 was the
    // one backend whose session did not have it — the other five load this
    // same file at engine startup. It travels the same way the operator
    // semantics do, for the same reason: there is no file to open at runtime.
    env.add_global(
        "json_builtins_lua_c",
        c_lua_dostring_chunks(JSON_BUILTINS_LUA),
    );
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    env.add_filter("escape_c", escape_c);
    env.add_filter("escape_json_string", escape_json_string);
    register_ecmascript_filters(env, scope);
    env.add_filter("to_in_predicate_c11", to_in_predicate_c11);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("read_data_src", read_data_src);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
    // SCE Protocol-Synthesis RFC §synth-5-E per-link delivery codegen: per-link function names
    // (`<machine>_deliver_link_<X>_sample`) snake-case the link
    // name to keep the C identifier stable when the SCXML link
    // attribute uses kebab-case or mixedCase.
    env.add_filter("to_snake_case", to_snake_case);
}

/// Escape C string literals (identical escaping rules to Rust/C++).
fn escape_c(text: String) -> String {
    escape_rust(text)
}

/// The shared §scxml-B-2 operator semantics, bound at compile time.
///
/// Same file the C++, Rust, Go, Kotlin and Python engines load; the C11
/// backend cannot read it at runtime, so codegen carries it instead.
pub const ECMA_SEMANTICS_LUA: &str = include_str!("../../sce/include/scripting/ecma_semantics.lua");

/// The shared §scxml-B-2 `JSON` builtins, bound at compile time.
///
/// Same file the C++, Rust, Go and Kotlin Lua engines load; the C11 backend
/// cannot read it at runtime, so codegen carries it instead. `JSON.stringify`
/// is what every backend's structured `<data>` reader asks the engine for,
/// and a C11 session without it would be the one backend that could not
/// answer.
pub const JSON_BUILTINS_LUA: &str = include_str!("../../sce/include/scripting/json_builtins.lua");

/// Render a Lua source file as a sequence of `luaL_dostring` calls, each
/// carrying a string literal short enough for a conforming C compiler.
///
/// ISO C99 §5.2.4.1 guarantees only 4095 characters in a string literal,
/// and GCC's `-Woverlength-strings` (which `-pedantic -Werror` turns fatal
/// in the C11 test build) measures the length *after* adjacent literals are
/// concatenated — so splitting one literal per source line does not help.
/// Several `luaL_dostring` calls do, because each is a complete Lua chunk.
///
/// The split points are blank lines, which in this file separate top-level
/// definitions. That is what obliges the file to have no file-local
/// helpers: a `local function` would not be visible to the chunk that came
/// after it. `ecma_semantics.lua` says so at the definition that used to be
/// one.
/// Split shared Lua source into the pieces the C11 embed loads separately.
///
/// The split is on BLANK LINES, which is what keeps a chunk a complete Lua
/// chunk: every definition in the shared assets is a top-level `function`
/// separated from its neighbours by one, so a boundary never falls inside a
/// body. That property is load-bearing rather than tidy — the generated code
/// discards each `luaL_dostring` result, so a chunk that failed to compile
/// would leave its definitions simply absent, and the first document to call
/// one would fail at runtime with no reference back to here.
///
/// Public so the split can be exercised against a real interpreter rather
/// than reviewed: `sce-build/tests/shared_lua_assets.rs` loads these chunks
/// one at a time into one state and then calls what they define.
pub fn c11_lua_chunks(text: &str) -> Vec<String> {
    // Well under the 4095 floor, leaving room for the escaping expansion of
    // whatever a maintainer adds to a paragraph.
    const MAX_CHUNK: usize = 2500;
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for paragraph in text.split("\n\n") {
        if !current.is_empty() && current.len() + paragraph.len() > MAX_CHUNK {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(paragraph);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn c_lua_dostring_chunks(text: &str) -> String {
    c11_lua_chunks(text)
        .iter()
        .map(|chunk| {
            let literal = chunk
                .lines()
                .map(|line| format!("\"{}\\n\"", escape_rust(line.to_string())))
                .collect::<Vec<_>>()
                .join("\n            ");
            format!("        (void)luaL_dostring(sm->L,\n            {literal});")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Escape characters for embedding inside a Lua short string literal
/// (Lua 5.4 §3.1). Only the inner escapes are produced — the surrounding
/// `"..."` quotes belong to the Lua source the template assembles.
///
/// This is a *different layer* from `escape_rust` / `escape_go` /
/// `escape_c` even though the rule sets coincide. Those escape a value
/// into the host language's literal; this one escapes a value into the
/// Lua source that the host literal *carries*. The Rust/Go `<send>`
/// backends embed a Lua table constructor
/// (`{ ["name"]="value" }`) inside a host string literal and hand it to
/// the script engine, so an author value reaching that site crosses two
/// literal boundaries and must be escaped once per boundary:
/// `static_value | escape_lua | escape_rust`. Applying only the host
/// filter yields host source that compiles but Lua source that does not
/// parse — the same composed shape `escape_json_string | escape_c`
/// already uses for the C11 JSON wire form.
///
/// The replace order is fixed: backslash first so the `\` introduced by
/// the subsequent escapes is not double-escaped. A raw newline is
/// illegal inside a Lua short literal, so `\n`/`\r` are mandatory rather
/// than cosmetic.
pub fn escape_lua(text: String) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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

/// §scxml-5.9.2: rewrite pure In('xxx') predicate text to a C11 native
/// `<machine>_in_state(sm, <MACHINE>_STATE_<XXX>)` call. Mirrors cpp
/// `parser::convert_in_to_cpp` which substitutes `this->isStateActive("xxx")`
/// — both sit at the codegen-time-text-substitution layer (inline-only
/// for C11) instead of routing through a runtime `In()` callback.
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
///
/// Carries the scope for the same single filter, and for the same reason,
/// as [`register_cpp_filters`].
pub fn register_kotlin_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    let for_content = Arc::clone(scope);
    env.add_filter("to_author_data_content", move |content: String| {
        to_author_data_content(content, &for_content)
    });
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_camel_case", to_camel_case);
    env.add_filter("escape_kotlin", escape_kotlin);
    env.add_filter("to_kotlin_string_expr", to_kotlin_string_expr);
    env.add_filter("to_event_class_name", to_event_class_name);
    env.add_filter("to_state_class_name", to_state_class_name);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
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
        let inner = inner
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$");
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
                .map(|p| {
                    if p.is_empty() {
                        String::new()
                    } else {
                        capitalize_first(p)
                    }
                })
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
/// Python AOT: state/event variants are `IntEnum` members named
/// in UPPER_SNAKE_CASE (`State.IDLE`, `Event.TEMP_HIGH`). Identifiers from
/// SCXML are normalised via `to_python_const`; script bodies are emitted
/// as repr-quoted Python string literals via `py_string_literal` so the
/// template never has to worry about embedded quotes / newlines / unicode.
/// A `PYTHON_KEYWORDS` / `escape_python_keyword` pair is intentionally
/// absent — every emitted identifier is UPPER_SNAKE_CASE (IntEnum
/// member) so keyword collisions cannot occur at this layer, and
/// datamodel variables live inside the `IScriptEngine` session behind
/// string-keyed accessors, so SCXML author identifiers never reach a
/// Python parser either.
pub fn register_python_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    env.add_filter("w3c_session_name", w3c_session_name);
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
    register_ecmascript_filters(env, scope);
    env.add_filter("to_event_variant", to_event_variant);
    env.add_filter("to_state_variant", to_state_variant);
    env.add_filter("to_machine_name", to_pascal_case);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
    // §scxml-5.2.2: `<data src="file:...">` inlining at codegen time —
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
