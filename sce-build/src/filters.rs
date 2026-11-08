// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Jinja2 template filters — multi-language filter registry.
// Rust, C++, and Kotlin filters registered with minijinja for template rendering.

use minijinja::Value;
use regex::Regex;
use std::borrow::Cow;
use std::sync::{Arc, LazyLock};

use crate::ecmascript::DocumentScope;

// ── Compiled regex patterns (compiled once, reused across calls) ──

/// Splits on dot, underscore, or hyphen — used for PascalCase, camelCase, event variants.
static RE_DELIMITERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[._\-]").unwrap());

/// Splits on underscore or hyphen only (no dot) — used for state variants, state/event class names.
static RE_WORD_DELIMITERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[_\-]").unwrap());

/// Matches C++ `isStateActive("stateId")` calls, for the Swift and Go
/// transformations of a pure `In()` condition.
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

/// The locations an `<assign>` may not write: the system variables the
/// specification binds for the session and keeps read-only.
///
/// A location is a literal, so whether an `<assign>` names one of these is
/// known when the machine is built, and every backend that refuses it there
/// reads this one list through the `read_only_location` template test
/// ([`is_read_only_location`], registered by `generator::new_env`). The paths
/// that decide it when the machine runs keep the same four: the C++ runtime's
/// `AssignHelper::isValidLocation` (the Interpreter) and the Python template's
/// `_READONLY_SYSTEM_VARS`.
pub const READ_ONLY_LOCATIONS: &[&str] = &["_sessionid", "_event", "_name", "_ioprocessors"];

/// `{% if action.location is read_only_location %}`: whether an `<assign>` to
/// `location` writes one of [`READ_ONLY_LOCATIONS`], and so raises
/// error.execution and ends its block instead of writing anything.
pub fn is_read_only_location(location: &str) -> bool {
    READ_ONLY_LOCATIONS.contains(&location)
}

/// Rust 2021 edition reserved keywords — must be escaped with `r#` prefix
pub(crate) const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", // Reserved for future use
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized",
    "virtual", "yield", "try", "union",
];

/// Register the string-literal escaper `syntax` needs, and nothing else.
///
/// The door that WRITES a filter registers it — the arrangement
/// `generator::register_template` already holds for `comment_text::FILTER`,
/// and for the same measured reason: the per-backend `register_*_filters`
/// functions are not the only way an environment comes to exist. Mesh builds
/// its own, and a rewritten literal there would fail to render with an
/// unknown filter rather than escaping anything.
///
/// Registering the same filter twice is harmless — minijinja replaces the
/// entry with an identical one — so this neither depends on nor disturbs the
/// per-backend registration.
pub fn register_literal_escaper(
    env: &mut minijinja::Environment,
    syntax: crate::template_lexing::Syntax,
) {
    use crate::template_lexing::Syntax;
    match syntax {
        Syntax::CFamily => env.add_filter("escape_cpp", escape_cpp),
        Syntax::Go => env.add_filter("escape_go", escape_go),
        Syntax::Kotlin => env.add_filter("escape_kotlin", escape_kotlin),
        Syntax::Python => env.add_filter("escape_python", escape_python),
        Syntax::Rust => env.add_filter("escape_rust", escape_rust),
        Syntax::Swift => env.add_filter("escape_swift", escape_swift),
        Syntax::Ttcn3 => env.add_filter("escape_ttcn3", escape_ttcn3),
    }
}

/// Register all Rust-specific filters on the minijinja environment.
pub fn register_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    register_event_wire_filters(env);
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
    let for_assign = Arc::clone(scope);
    env.add_filter("to_lua_assign_content", move |content: String| {
        to_lua_assign_content(content, &for_assign)
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
    // Whether the document declares a name: a `<data>`, or a location something writes
    // (`<assign>`, `<foreach item>`). A `<send namelist>` asks it of each name, because a name
    // that is not declared is an argument that cannot be evaluated (W3C test 553), and
    // `to_lua_expr` would refuse it at build time where every backend answers at run time.
    let for_declared = Arc::clone(scope);
    env.add_test("declared_name", move |name: String| {
        for_declared.declares(&name)
    });
    env.add_filter("escape_lua", escape_lua);
    let for_xml = Arc::clone(scope);
    env.add_filter("data_content_is_xml", move |content: String| {
        data_content_is_xml(content, &for_xml)
    });
    register_inline_content_reading(env, scope);
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

/// Escape the author's text for the body of a Rust string literal.
///
/// A backslash and a double quote are escaped because they end or bend the
/// literal; a line break, a carriage return (which rustc refuses bare in a
/// literal) and a tab take their named escapes; every other control character
/// is written `\u{…}`, so no generated line carries a byte a reader cannot see;
/// and so is each character that changes the direction text is shown in,
/// which rustc's `text_direction_codepoint_in_literal` lint refuses by default.
/// Every other character is itself.
pub fn escape_rust(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() || changes_text_direction(c) => {
                out.push_str(&format!("\\u{{{:x}}}", u32::from(c)))
            }
            c => out.push(c),
        }
    }
    out
}

/// Whether `c` is one of the characters that embed, override or isolate the
/// direction text is shown in (Unicode's bidirectional formatting characters),
/// or a directional mark: a reader of generated source cannot see one, and it
/// can make the code shown differ from the code compiled.
fn changes_text_direction(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
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

/// Convert dot-separated SCXML event name to Rust enum variant PascalCase:
/// [`to_pascal_case`], with a keyword escaped as [`pascal_member`] says.
pub fn to_event_variant(name: String) -> String {
    pascal_member(to_pascal_case(name))
}

/// A document name spelled PascalCase as the member of a generated type — a
/// Rust variant, a Kotlin object or enum entry, the tail of a Go constant —
/// with a keyword escaped.
///
/// A PascalCase spelling begins with a capital or a digit, so of the words
/// the backends that write it bare reserve (Rust and Kotlin), only Rust's
/// `Self` can be one: a state `self` was the variant `Self`, which does not
/// parse. `r#Self` is no identifier either (Rust refuses the raw form of
/// `self`, `Self`, `super` and `crate`), so the escape is a trailing `_`.
///
/// A spelling that is no identifier at all is made one by
/// [`joined_identifier`]: an event `1x` is `_1x`, a state `_` is `_Blank`.
///
/// No escape can meet another name: [`to_pascal_case`] drops every `_` it
/// reads, so no name of a document is spelled with one, and the three escapes
/// put it where no other does (after a word, before a digit, before
/// `Blank`). Every site that writes such a member calls this one spelling,
/// Go's and Kotlin's included, so a member is named alike where it is
/// declared and wherever it is used.
fn pascal_member(spelled: String) -> String {
    let reserved = |language| crate::reader_names::is_reserved_word(language, &spelled);
    if reserved(crate::generator::Language::Rust) || reserved(crate::generator::Language::Kotlin) {
        format!("{spelled}_")
    } else {
        joined_identifier(spelled, "Blank")
    }
}

/// A spelling that joins a name's words with nothing (PascalCase, lowerCamel)
/// as an identifier.
///
/// An event token may begin with a digit where an XML Name may not (an event
/// `1x`, the second token of `DIGIT.0`), and a state's first word may be `_`
/// and a digit (`_1`), so the joined spelling can begin with a digit: it is
/// written after a `_` (`_1x`). A name made of delimiters alone (`_`, `_-_`,
/// `_._`) has no word left and joins to nothing: it is `_` and `blank`. Two
/// such names are one member, and refused as any two names one backend folds
/// together are (`crate::member_names`). The joined spelling holds no `_`, so
/// neither escape is ever the spelling of another name.
fn joined_identifier(spelled: String, blank: &str) -> String {
    match spelled.chars().next() {
        None => format!("_{blank}"),
        Some(first) if first.is_ascii_digit() => format!("_{spelled}"),
        Some(_) => spelled,
    }
}

/// Convert SCXML state ID to Rust enum variant PascalCase.
///
/// Splits on `.` as well as `_` and `-`, exactly as an event variant does:
/// all three are legal in an XML Name and none in a Rust, Go or Kotlin
/// identifier. It used to split on `_` and `-` only, a leftover of byte
/// parity with an earlier generator, and a state `a.b` became the variant
/// `A.b` in Rust, Go and Kotlin, none of which compiled (measured
/// 2026-09-29; `sce-build/tests/generated_names.rs`). A keyword is escaped as
/// [`pascal_member`] says: a state `self` is the variant `Self_`.
pub fn to_state_variant(name: String) -> String {
    pascal_member(to_pascal_case(name))
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

/// §scxml-B-2: Normalize whitespace.
fn normalize_ws(text: String) -> String {
    RE_WHITESPACE.replace_all(text.trim(), " ").to_string()
}

/// §scxml-5.3.1: read external data file referenced by `<data src="...">`.
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
    // The path is the document's own (`<data src>`), so it is opened where a caller confined the
    // generator to a folder allows and not elsewhere (`SCE_FILE_ROOT`).
    crate::confine::read_to_string(&resolved).map_err(|e| {
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

/// The event data a `<send>`'s inline `<content>` hands its event as the text
/// itself, as the wire text: the XML as written, or otherwise the string the
/// text spells, whitespace-normalised and JSON-quoted.
///
/// That is the last two rungs of [`to_lua_data_content`], and the whole of the
/// reading under a data model with no value expressions to evaluate
/// ([`inline_content_reading`]): a `sce-static` document has none, so its
/// `<content>123</content>` is the string `"123"` on every backend.
pub fn static_content_wire(content: &str) -> String {
    if opens_as_xml(content) {
        return content.to_string();
    }
    format!(
        "\"{}\"",
        escape_json_string(normalize_ws(content.to_string()))
    )
}

/// Whether inline text is read as an XML document: its first non-blank
/// character is `<` (§scxml-B-2-8-1). The receiver builds the DOM from the text.
fn opens_as_xml(content: &str) -> bool {
    content.trim_start().starts_with('<')
}

/// What a `<send>`'s inline `<content>` text is as the event's data, decided at
/// build time from the text and the document's data model.
///
/// One answer for every backend's send template and for the analysis that says
/// whether the machine carries a script engine
/// ([`crate::script_engine_analyzer`]), so the reading follows from the data
/// model and the text alone. It used to follow from whether something ELSE in
/// the document needed an engine: under `datamodel="ecmascript"`,
/// `<content>21</content>` arrived as the number 21 in a machine that carried
/// one for another reason and as the string "21" in a machine that did not.
///
/// Serialized as `{kind, wire}`, so a template branches on `reading.kind` and
/// writes `reading.wire` into the literal the event's data is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "wire", rename_all = "snake_case")]
pub enum InlineContentReading {
    /// An XML document, carried as the text written: the receiver builds the
    /// DOM from it.
    Xml(String),
    /// A value the text fixes on its own, as the JSON the event carries: a
    /// literal under the ECMAScript data model, the string the text spells
    /// otherwise.
    Value(String),
    /// An expression the data model evaluates when the send runs, so the
    /// machine carries a script engine to evaluate it with.
    Expression,
}

impl InlineContentReading {
    /// Whether the event's data is computed when the send runs.
    pub fn is_evaluated(&self) -> bool {
        matches!(self, Self::Expression)
    }
}

/// How `content`, the inline text of a `<send>`'s `<content>`, becomes the
/// event's data under `datamodel` (§scxml-B-2-8-1, with `scope` the names the
/// document declares).
///
/// The ECMAScript data model reads the text first as an expression, then as
/// XML, then as the space-normalized string, which is the order
/// [`to_lua_data_content`] takes. An expression whose value needs nothing
/// bound, a literal ([`crate::ecmascript::constant_value`]), is that value
/// now; any other is evaluated when the send runs. With no value expression
/// language to read the text in (`null`, and `sce-static`, whose expressions
/// are not ECMAScript), the text is the value: the XML as written or the
/// normalized string ([`static_content_wire`]).
pub fn inline_content_reading(
    content: &str,
    datamodel: crate::model::Datamodel,
    scope: &DocumentScope,
) -> InlineContentReading {
    if datamodel == crate::model::Datamodel::EcmaScript
        && crate::ecmascript::to_lua_value(content, scope).is_ok()
    {
        return match crate::ecmascript::constant_value(content) {
            Some(value) => InlineContentReading::Value(json_wire_text(&value)),
            None => InlineContentReading::Expression,
        };
    }
    let wire = static_content_wire(content);
    if opens_as_xml(content) {
        InlineContentReading::Xml(wire)
    } else {
        InlineContentReading::Value(wire)
    }
}

/// A JSON value as the text every engine writes it in: members in ascending
/// key order (`tests/json_text/object_key_order.json`), strings escaped by
/// [`escape_json_string`] and numbers spelled by [`json_number_text`].
fn json_wire_text(value: &serde_json::Value) -> String {
    use serde_json::Value as J;
    match value {
        J::Null => "null".to_string(),
        J::Bool(b) => b.to_string(),
        J::Number(n) => n
            .as_f64()
            .map_or_else(|| "null".to_string(), json_number_text),
        J::String(s) => format!("\"{}\"", escape_json_string(s.clone())),
        J::Array(items) => {
            let parts: Vec<String> = items.iter().map(json_wire_text).collect();
            format!("[{}]", parts.join(","))
        }
        J::Object(members) => {
            let mut keys: Vec<&String> = members.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "\"{}\":{}",
                        escape_json_string(key.clone()),
                        json_wire_text(&members[key.as_str()])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

/// A finite double as ECMAScript's Number::toString spells it in radix 10
/// (ARCHITECTURE.md, "JSON Number Text"; `tests/json_text/real_text.json`
/// holds the cases): the fewest digits that read back as the same double, in
/// decimal notation when `1e-6 <= |x| < 1e21` and as `d[.ddd]e[+-]n`
/// otherwise, and `0` for either zero.
///
/// Rust writes the same fewest digits for both of its notations, `{}` without
/// an exponent and `{:e}` with one; only the sign of a positive exponent is
/// ECMAScript's to add.
fn json_number_text(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if (1e-6..1e21).contains(&value.abs()) {
        return format!("{value}");
    }
    let scientific = format!("{value:e}");
    match scientific.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => scientific,
    }
}

/// The value an `<assign>`'s in-line children specify, as an expression.
///
/// The ordering is [`to_lua_data_content`]'s and only the last rung
/// differs. `<data>` hands XML back unlowered because the initializer at
/// the other end of that seam builds a DOM node from it; an assignment has
/// no such end. Every backend already resolved that the same way — c11,
/// cpp, kotlin, rust and go each wrapped the serialised XML in a
/// single-quoted string literal, and python's runtime fallback arrived at
/// the same string after its eval failed — so the rung is written down
/// here once instead of being re-derived five times in Jinja.
///
/// # Why the last rung is a string and not a DOM
///
/// Because the Recommendation asks for a DOM in exactly two places and
/// this is neither of them. The ECMAScript data model appendix gives
/// `<data>`'s in-line content and `src` the JSON → XML-DOM →
/// space-normalized-string ladder, and gives an arriving `_event.data`
/// the same one. An `<assign>`'s children are, in the words of the
/// element's own clause, an "in-line specification of the legal data
/// value" — which points at the section defining what a legal value *is*,
/// and there is no ladder in it. The appendix's own `<assign>` entry names
/// only `expr`.
///
/// So the string reading here is conformant, and W3C test530 depends on
/// it: it assigns a serialised `<scxml>` document and invokes it through
/// `<content expr>`, which wants the text back. An earlier revision of
/// this comment asserted the opposite — that the appendix's ladder
/// covered these children and the unanimous backend answer therefore
/// violated it — and recorded the asymmetry as a debt. It was a
/// misreading of the appendix, and it cost two rounds of registry
/// priority before anyone read the `<assign>` entry there.
///
/// What this removes is the rung *above* it. An `<assign>` whose child is
/// text — `21`, `{ "k": 7 }`, `hello there` — was quoted by those same
/// five backends, so `21` reached the datamodel as the string "21" and a
/// `cond="v == 21"` guarding it was false. With the reading made here, the
/// content branch and the `expr` branch produce the same kind of thing:
/// two spellings of one value, which is what "either … but not both"
/// means.
pub fn to_lua_assign_content(
    content: String,
    scope: &DocumentScope,
) -> Result<String, minijinja::Error> {
    if content.is_empty() {
        return Ok(String::new());
    }
    // §scxml-5.4: an `<assign>`'s children specify the value its 'expr'
    // would have, so the expression reading is tried first and the text
    // reading is what it falls to.
    if let Ok(lua) = crate::ecmascript::to_lua_value(&content, scope) {
        return Ok(lua);
    }
    Ok(lua_string_literal(&normalize_ws(content)))
}

/// The same value for the two backends that hand the author's ECMAScript
/// to an ECMAScript engine rather than lowering it.
///
/// [`to_author_data_content`] is to [`to_lua_data_content`] what this is
/// to [`to_lua_assign_content`], and for the reason stated there: whether
/// the text is an expression at all is a question about the document, so
/// it is asked once and answered the same way for every backend. Only the
/// spelling of the answer is language-specific.
pub fn to_author_assign_content(
    content: String,
    scope: &DocumentScope,
) -> Result<String, minijinja::Error> {
    if content.is_empty() {
        return Ok(String::new());
    }
    // §scxml-5.4: the same question about the same children, answered in
    // the language this backend's engine already speaks.
    if crate::ecmascript::to_lua_value(&content, scope).is_ok() {
        return Ok(content);
    }
    Ok(ecmascript_string_literal(&normalize_ws(content)))
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
///
/// A control character, and U+2028 and U+2029 (which end a line wherever an
/// engine older than ECMA-262 2019 reads one), are written `\uXXXX`. The
/// forge expression emitter writes a string literal for the Interpreter
/// through this too (`crate::forge::expr`), so the two cannot spell a value
/// two ways.
pub(crate) fn ecmascript_string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)))
            }
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

/// Keep the `Invoke::Unsupported` entries the host did NOT declare — the
/// ones that still lower to a §scxml-6.4.1 `error.execution` and start
/// nothing.
///
/// Paired with [`filter_host_invoker`], and the pair partitions
/// [`filter_unsupported`] exactly: every `<invoke>` whose type SCE does
/// not implement is lowered by one arm or the other, so a declaration
/// cannot leave a site with no lowering at all.
///
/// Deliberately a THIRD filter rather than a narrowing of `unsupported`.
/// That name gates the surrounding invoke machinery in eight places (the
/// pending-invoke field, the entry chain, the execute method, the exit
/// chain) and is read by five other backends. Narrowing it would have
/// made a declaration silently delete the machinery its own dispatch
/// needs — measured: the first attempt emitted neither arm, because the
/// method holding them was guarded by the name that had just stopped
/// matching.
fn filter_unsupported_refused(value: Value) -> Result<Value, minijinja::Error> {
    let all = filter_invokes_by_kind(value, "Unsupported")?;
    keep_invokes_where(all, |item| !invoke_is_host_served(item))
}

/// Keep only `Invoke::Unsupported` entries the host has DECLARED it can
/// run (§scxml-6.4.1 leaves the invokable set to the platform).
///
/// A backend with no invoker registry never sees an entry here — the
/// generator refuses the declaration for it by name before rendering.
fn filter_host_invoker(value: Value) -> Result<Value, minijinja::Error> {
    let all = filter_invokes_by_kind(value, "Unsupported")?;
    keep_invokes_where(all, invoke_is_host_served)
}

/// Does this `<invoke>` entry carry `host_served: true`?
fn invoke_is_host_served(item: &Value) -> bool {
    item.get_attr("host_served")
        .ok()
        .and_then(|v| v.is_true().then_some(true))
        .unwrap_or(false)
}

/// Filter an already-kind-selected invoke list by a predicate.
fn keep_invokes_where(
    value: Value,
    keep: impl Fn(&Value) -> bool,
) -> Result<Value, minijinja::Error> {
    let mut out: Vec<Value> = Vec::new();
    for item in value.try_iter()? {
        if keep(&item) {
            out.push(item.clone());
        }
    }
    Ok(Value::from(out))
}

/// Register the variant-filtering helpers shared by every backend.
/// Called from each per-language `register_*_filters` function so the same
/// filter names are available no matter which template engine is rendering.
/// Does [`to_lua_data_content`] read this text as XML?
///
/// The three readings §scxml-B-2 orders are decided at codegen time, and a
/// payload's wire form differs between them: an expression is evaluated and
/// serialized, plain text becomes a JSON string, and XML travels as the
/// document text so the RECEIVER builds the DOM (W3C test561 — a sender that
/// serialized a parsed DOM would decide the reading for the far end).
///
/// Asking the same function rather than re-deriving the test is the point:
/// two spellings of "is this XML" would eventually disagree about a document
/// whose first non-space character is `<` but which parses as an expression.
/// The function is [`inline_content_reading`] under the ECMAScript data model,
/// which is the one inline `<data>` text is read in.
fn data_content_is_xml(content: String, scope: &DocumentScope) -> bool {
    matches!(
        inline_content_reading(&content, crate::model::Datamodel::EcmaScript, scope),
        InlineContentReading::Xml(_)
    )
}

/// `{{ action.content | inline_content_reading(model.datamodel) }}`: the
/// reading of a `<send>`'s inline `<content>` ([`inline_content_reading`]), as
/// the `{kind, wire}` object a template branches on.
///
/// The data model is the template's argument because the scope is the one
/// thing the environment holds about the document; an attribute spelling SCE
/// does not accept is refused rather than read as some default.
fn register_inline_content_reading(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    let for_reading = Arc::clone(scope);
    env.add_filter(
        "inline_content_reading",
        move |content: String, datamodel: String| -> Result<Value, minijinja::Error> {
            let datamodel = crate::model::Datamodel::from_attr(&datamodel).ok_or_else(|| {
                minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    format!(
                        "inline_content_reading: `{datamodel}` names no data model SCE accepts"
                    ),
                )
            })?;
            Ok(Value::from_serialize(inline_content_reading(
                &content,
                datamodel,
                &for_reading,
            )))
        },
    );
}

/// The filters that build an event payload's **wire** form.
///
/// The clause cited in the body names JSON as the serialization for a value
/// that leaves the ECMAScript data model, and an event payload always leaves
/// it: the reader is another dequeue, often another session, and — in a mesh —
/// another process running another backend. Every backend registers these
/// because the wire is the one thing they must agree on; a backend that
/// spelled the payload in its own engine's source language made `_event.data`
/// mean whatever the *sender* was written in, and made the payload executable
/// at the receiving end.
pub fn register_event_wire_filters(env: &mut minijinja::Environment) {
    // §scxml-B-2-9: a value that leaves the ECMAScript data model is
    // serialized to JSON.
    env.add_filter("escape_json_string", escape_json_string);
    env.add_filter("static_params_json", static_params_json);
    env.add_filter("event_wire_name", event_wire_name);
}

/// The name an entry of the document's event set goes by in a generated name
/// table: its own, except the wildcard entry, which is the descriptor that asks
/// for it ([`WILDCARD_EVENT_WIRE_NAME`]). The table that maps `*` to the member
/// is what a name that no event of the document matches is delivered through.
fn event_wire_name(name: String) -> String {
    if name == crate::analyzer::WILDCARD_EVENT {
        crate::analyzer::WILDCARD_EVENT_WIRE_NAME.to_string()
    } else {
        name
    }
}

/// A `<send>`'s params, as JSON, when every value is a literal.
///
/// The dynamic path evaluates each param and hands the values to the runtime
/// helper each backend ports from `EventDataHelper::buildJsonFromTypedParams`.
/// This is the path where there is nothing to evaluate — and it exists
/// because that machine may have no script engine at all (measured: the
/// `send_param_payload` synth-invoke emitter has none), so the payload has to
/// be finished here or not at all. The Lua-source wire could not do this: it
/// left an interpreter's worth of work for the far end.
///
/// The duplicate-name rule is the same one the runtime helpers apply, and
/// `static_params_json_matches_the_runtime_helper` in this crate's tests runs
/// both against one param list rather than trusting the two prose copies.
fn static_params_json(value: Value) -> Result<String, minijinja::Error> {
    // §scxml-6.2: the `<param>` elements a `<send>` carries become the data
    // the receiving event exposes.
    //
    // Sorted by name, which is what the runtime helpers do (`BTreeMap` there,
    // `std::map` in the C++ original) — the two paths have to produce the same
    // bytes for the same params or a machine's payload would depend on whether
    // its values happened to fold. Within one name the values keep document
    // order, because W3C test178 requires every value of a repeated name
    // delivered and their order is the only thing distinguishing them.
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for item in value.try_iter()? {
        let name = item
            .get_attr("name")
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let literal = item
            .get_attr("static_value")
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        groups.entry(name).or_default().push(literal);
    }
    let mut parts: Vec<String> = Vec::new();
    for (name, values) in &groups {
        // A single occurrence is the value; more than one is an Array of
        // them. An object cannot hold one name twice.
        let published = if values.len() == 1 {
            format!("\"{}\"", escape_json_string(values[0].clone()))
        } else {
            let items: Vec<String> = values
                .iter()
                .map(|v| format!("\"{}\"", escape_json_string(v.clone())))
                .collect();
            format!("[{}]", items.join(","))
        };
        parts.push(format!(
            "\"{}\":{}",
            escape_json_string(name.clone()),
            published
        ));
    }
    Ok(format!("{{{}}}", parts.join(",")))
}

pub fn register_invoke_filters(env: &mut minijinja::Environment) {
    env.add_filter("scxml", filter_scxml);
    env.add_filter("scxml_remote", filter_scxml_remote);
    env.add_filter("hybrid", filter_hybrid);
    env.add_filter("scxml_family", filter_scxml_family);
    env.add_filter("mesh_rpc", filter_mesh_rpc);
    env.add_filter("unsupported", filter_unsupported);
    env.add_filter("unsupported_refused", filter_unsupported_refused);
    env.add_filter("host_invoker", filter_host_invoker);
    env.add_filter("candidate_identifier", filter_candidate_identifier);
}

/// `{{ invoke.field_suffix | candidate_identifier(candidate.stem) }}`: the
/// identifier a template builds a candidate's names from — the one derivation,
/// [`crate::model::invoke_candidate_identifier`], read where the template is.
fn filter_candidate_identifier(invoke_suffix: String, candidate_stem: String) -> String {
    crate::model::invoke_candidate_identifier(&invoke_suffix, &candidate_stem)
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

// ── Go filters ──────────────────────────────────────────────────

/// Go reserved keywords — must be escaped with `_` suffix
pub(crate) const GO_KEYWORDS: &[&str] = &[
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
    register_event_wire_filters(env);
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

/// Register all Swift-specific filters on the minijinja environment.
///
/// A state or an event is a case of the policy's own enum, spelled lowerCamel as
/// every Swift enum case is, with a keyword escaped ([`to_swift_variant`]). No
/// separate keyword filter is registered: the spelling filter is the escape, so
/// a template cannot write the case one way and name it another.
pub fn register_swift_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    // The build-time ECMAScript to Lua lowering, which the `SceLua` module's engine runs
    // (docs/SCE_LUA_TRANSLATION_SEAM.md): the same family every other Lua backend registers.
    register_ecmascript_filters(env, scope);
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    register_event_wire_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_camel_case", to_camel_case);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_event_variant", to_swift_variant);
    env.add_filter("to_state_variant", to_swift_variant);
    env.add_filter("to_in_predicate_swift", to_in_predicate_swift);
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("to_machine_name", to_pascal_case);
    // Whether an inline `<content>` is XML, which travels as the document text
    // (§scxml-B-2-8-1) and not as a JSON string.
    let for_xml = Arc::clone(scope);
    env.add_filter("data_content_is_xml", move |content: String| {
        data_content_is_xml(content, &for_xml)
    });
    // §scxml-5.3.1: the text a `<data src>` names, read when the machine is generated, so the
    // machine carries no path and opens no file (the C11 and Python backends do the same).
    env.add_filter("read_data_src", read_data_src);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
}

/// A state id or an event name as the case of a Swift enum: lowerCamel, the
/// spelling `crate::scxml_identifier` and `crate::reader_names` hold a document
/// name against for this backend.
///
/// A name Swift reserves is written in backticks, which declare it and name it
/// again wherever the template writes it (`case `break``, `.`break``): an event a
/// document calls `break` is a case, not a refusal.
///
/// Except the words in [`SWIFT_WORDS_AFTER_A_DOT`]: the template names a case
/// after a `.` (`.idle`, `State.idle`), and there Swift's grammar gives each of
/// them a meaning of its own, which a backtick is not known to reach (an enum
/// case `init` is read as the initializer where a leading dot names it). Each
/// is spelled with a trailing `_`, a name and no word: a state `self` is the
/// case `self_`. [`to_camel_case`] drops every `_` it reads, so no other name
/// of a document is spelled so, and the escape cannot meet another case.
///
/// A spelling that is no identifier at all is made one as
/// [`joined_identifier`] says: an event `1x` is the case `_1x`, a state `_`
/// the case `_blank`.
pub fn to_swift_variant(name: String) -> String {
    let spelled = to_camel_case(name);
    if SWIFT_WORDS_AFTER_A_DOT.contains(&spelled.as_str()) {
        format!("{spelled}_")
    } else if crate::reader_names::is_reserved_word(crate::generator::Language::Swift, &spelled) {
        format!("`{spelled}`")
    } else {
        joined_identifier(spelled, "blank")
    }
}

/// The reserved words Swift's grammar reads after a `.`: `.self` (a postfix
/// self expression), `.init` (an initializer expression), `.Type` and
/// `.Protocol` (a metatype), and `Self`, the type an enum's own body names
/// itself by.
const SWIFT_WORDS_AFTER_A_DOT: &[&str] = &["Protocol", "Self", "Type", "init", "self"];

/// Transform C++ In() predicate code to Swift: a call on the policy itself.
fn to_in_predicate_swift(cond_cpp: String) -> String {
    if cond_cpp.is_empty() {
        return String::new();
    }
    let result = cond_cpp.replace("this->", "");
    RE_IS_STATE_ACTIVE
        .replace_all(&result, r#"isStateActive("$1")"#)
        .to_string()
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

/// Escape the author's text for the body of a Go interpreted string literal.
///
/// A backslash and a double quote are escaped, a line break, a carriage return
/// and a tab take their named escapes, and every other control character is
/// written `\uXXXX`: the Go compiler refuses a NUL in source, and a `\xHH`
/// would be one byte, not the character, past 7F. A byte order mark is written
/// `﻿` too, since the compiler refuses one anywhere but at the start of a
/// file; and so is a character that changes the direction text is shown in.
pub(crate) fn escape_go(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || c == '\u{FEFF}' || changes_text_direction(c) => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)))
            }
            c => out.push(c),
        }
    }
    out
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
/// The C++ seam filters: one call emits a whole `SCE::ScriptSource`.
///
/// The pair cannot be assembled at the template site. A site that spelled
/// `ScriptSource::lua("{{ e | to_lua_expr }}", "{{ e | escape_cpp }}")` by
/// hand would be one edit away from lowering one half and not the other, and
/// the resulting artifact would name a language the author never wrote in
/// every diagnostic it raised — the failure
/// `docs/SCE_LUA_TRANSLATION_SEAM.md` calls "the seam cannot be a one-string
/// signature". So the filter emits BOTH halves, or neither.
///
/// Which form comes out is the run's `--script-engine` selection, closed over
/// here rather than read from the render context: a template that could ask
/// would be a template that could ask *inconsistently*.
fn register_script_source_filters(
    env: &mut minijinja::Environment,
    scope: &Arc<DocumentScope>,
    target: crate::generator::ScriptEngineTarget,
    spelling: ScriptSourceSpelling,
) {
    use crate::generator::ScriptEngineTarget;
    let script_source_ecmascript = move |source: &str| -> String { spelling.ecmascript(source) };
    let script_source_lua =
        move |lowered: &str, source: &str| -> String { spelling.lua(lowered, source) };

    // Expressions and guards differ only in the lowering they take: a guard
    // goes through the truthiness helper, an expression does not. Under the
    // ECMAScript target the distinction disappears, because the author's text
    // is the answer either way.
    // The lowering half is fallible — the frontend refuses what it cannot
    // parse — and that refusal is propagated rather than swallowed. A filter
    // that fell back to the author's text on a parse error would emit an
    // artifact half in each language and say nothing.
    let for_expr = Arc::clone(scope);
    env.add_filter(
        "to_script_source_expr",
        move |expr: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => script_source_ecmascript(&expr),
                ScriptEngineTarget::Lua => {
                    script_source_lua(&to_lua_expr(expr.clone(), &for_expr)?, &expr)
                }
            })
        },
    );
    let for_guard = Arc::clone(scope);
    env.add_filter(
        "to_script_source_guard",
        move |expr: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => script_source_ecmascript(&expr),
                ScriptEngineTarget::Lua => {
                    script_source_lua(&to_lua_guard(expr.clone(), &for_guard)?, &expr)
                }
            })
        },
    );
    let for_script = Arc::clone(scope);
    env.add_filter(
        "to_script_source_script",
        move |body: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => script_source_ecmascript(&body),
                ScriptEngineTarget::Lua => {
                    script_source_lua(&to_lua_script(body.clone(), &for_script)?, &body)
                }
            })
        },
    );
    // Inline `<content>` for `<data>` / `<send>`, and for `<assign>`.
    //
    // Content has its own pair of lowerings already — `to_author_*` for the
    // backends that hand over source, `to_lua_*` for the ones that lower —
    // and these are the seam's spelling of that same split: one call picks the
    // arm the run selected and carries both halves out of it.
    let for_data_content = Arc::clone(scope);
    env.add_filter(
        "to_script_source_data_content",
        move |content: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => {
                    script_source_ecmascript(&to_author_data_content(content, &for_data_content)?)
                }
                ScriptEngineTarget::Lua => script_source_lua(
                    &to_lua_data_content(content.clone(), &for_data_content)?,
                    &to_author_data_content(content, &for_data_content)?,
                ),
            })
        },
    );
    let for_assign_content = Arc::clone(scope);
    env.add_filter(
        "to_script_source_assign_content",
        move |content: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => script_source_ecmascript(
                    &to_author_assign_content(content, &for_assign_content)?,
                ),
                ScriptEngineTarget::Lua => script_source_lua(
                    &to_lua_assign_content(content.clone(), &for_assign_content)?,
                    &to_author_assign_content(content, &for_assign_content)?,
                ),
            })
        },
    );
    // An assignment TARGET.
    //
    // Not an expression: a location names a place to write, and the frontend
    // lowers it with `to_lua_location` rather than `to_lua_expr` — a write is
    // how this datamodel's globals come into existence, so the target is not
    // resolved against what the document declares. It still crosses the
    // boundary as executable text (`AssignmentExecutionHelper` glues it in
    // front of `=` and runs the result), which is what puts it here at all.
    env.add_filter(
        "to_script_source_location",
        move |location: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => script_source_ecmascript(&location),
                ScriptEngineTarget::Lua => {
                    script_source_lua(&to_lua_location(location.clone())?, &location)
                }
            })
        },
    );
    // `<name> = <expr>`, executed as a script.
    //
    // A COMPOSED unit, and the reason it is a filter rather than two
    // interpolations glued at the site: under the Lua target the evaluated
    // half must join the LOWERED expression while the authored half joins the
    // author's, and a site spelling that by hand is one edit away from
    // lowering one of them and not the other. The location is a name — the
    // same name in either language — so it lands in both halves.
    let for_assign = Arc::clone(scope);
    env.add_filter(
        "to_script_source_assignment",
        move |expr: String, location: String| -> Result<String, minijinja::Error> {
            Ok(match target {
                ScriptEngineTarget::EcmaScript => {
                    script_source_ecmascript(&format!("{location} = {expr}"))
                }
                ScriptEngineTarget::Lua => script_source_lua(
                    &format!("{location} = {}", to_lua_expr(expr.clone(), &for_assign)?),
                    &format!("{location} = {expr}"),
                ),
            })
        },
    );
}

/// How one backend spells a `ScriptSource` in its own language.
///
/// The seam is the SAME on both sides — which lowering each filter applies,
/// which arm the run's `--script-engine` selection takes, and the rule that a
/// filter emits BOTH halves or neither. Only the constructor's spelling and
/// the string escaping differ, so those are what this carries and everything
/// else stays one implementation. A second copy of
/// [`register_script_source_filters`] per backend would be a second answer to
/// "what does the Lua target do with an `<assign>` location", free to drift
/// from the first the way the manifest's `script_engine_language` already did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptSourceSpelling {
    /// `::SCE::ScriptSource::…`, escaped for a C++ string literal.
    Cpp,
    /// `com.sce.runtime.ScriptSource.…`, escaped for a Kotlin string literal.
    ///
    /// Fully qualified because the generated machine is emitted into the
    /// document's own package and does not import the runtime wholesale —
    /// every other runtime type it names is spelled out the same way.
    Kotlin,
}

impl ScriptSourceSpelling {
    /// `ScriptSource::ecmascript("…")` — the author's text, unlowered.
    fn ecmascript(self, source: &str) -> String {
        match self {
            Self::Cpp => format!(
                "::SCE::ScriptSource::ecmascript(\"{}\")",
                escape_cpp(source.to_string())
            ),
            Self::Kotlin => format!(
                "com.sce.runtime.ScriptSource.ecmascript(\"{}\")",
                escape_kotlin(source.to_string())
            ),
        }
    }

    /// `ScriptSource::lua("…lowered…", "…source…")` — both halves, in the
    /// order the named constructor takes them.
    fn lua(self, lowered: &str, source: &str) -> String {
        match self {
            Self::Cpp => format!(
                "::SCE::ScriptSource::lua(\"{}\", \"{}\")",
                escape_cpp(lowered.to_string()),
                escape_cpp(source.to_string())
            ),
            Self::Kotlin => format!(
                "com.sce.runtime.ScriptSource.lua(\"{}\", \"{}\")",
                escape_kotlin(lowered.to_string()),
                escape_kotlin(source.to_string())
            ),
        }
    }
}

pub fn register_cpp_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    register_cpp_filters_for_engine(
        env,
        scope,
        crate::generator::Language::Cpp.default_script_engine_target(),
    )
}

/// [`register_cpp_filters`] with the run's engine selection named.
///
/// The two-function shape rather than a defaulted argument is the same one
/// `generate` / `generate_with_options` already uses next door: Rust has no
/// default arguments, and every existing caller means the default.
pub fn register_cpp_filters_for_engine(
    env: &mut minijinja::Environment,
    scope: &Arc<DocumentScope>,
    target: crate::generator::ScriptEngineTarget,
) {
    register_script_source_filters(env, scope, target, ScriptSourceSpelling::Cpp);
    register_cpp_filters_inner(env, scope)
}

fn register_cpp_filters_inner(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    register_inline_content_reading(env, scope);
    let for_content = Arc::clone(scope);
    env.add_filter("to_author_data_content", move |content: String| {
        to_author_data_content(content, &for_content)
    });
    let for_assign = Arc::clone(scope);
    env.add_filter("to_author_assign_content", move |content: String| {
        to_author_assign_content(content, &for_assign)
    });
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    register_event_wire_filters(env);
    env.add_filter("capitalize", capitalize_state);
    env.add_filter("to_cpp_event_variant", to_cpp_event_variant);
    // The spelling the typed payload channel names an event's tag value and
    // its payload struct by (`build_cpp_event_payload`), which is not the
    // event enum's.
    env.add_filter("to_event_variant", to_event_variant);
    // The spelling a `sce-static` variable's member is derived from
    // (`CppTarget::field_name`), for the parent that names it in a child's
    // `InvokeParams`.
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("escape_cpp", escape_cpp);
    env.add_filter("escape_cpp_format", escape_cpp_format);
    register_literal_door(env, crate::generator::Language::Cpp);
    // The whitespace a `sce-static` machine's literal `<send><content>` is
    // normalised by, as the other backends spell it.
    env.add_filter("normalize_ws", normalize_ws);
    env.add_filter("split", filter_split);
    env.add_filter("slice_from", filter_slice_from);
    env.add_filter("extern_callback_path", filter_extern_callback_path);
    env.add_filter("callback_lang", filter_callback_lang);
}

/// Capitalize state/event names for C++ enums.
///
/// `.` and `-` are written `_` first: both are legal in an XML Name and in
/// an event name (§scxml-3.12.1), and neither in a C++ identifier. Event
/// names always had this; state ids did not, and a state `door-open` was
/// declared `Door-open`, which clang-format then printed as `Door - open`
/// (measured 2026-09-29; `sce-build/tests/generated_names.rs`).
///
/// A spelling a C++ translation unit cannot declare is written after an `m`
/// ([`cpp_member`]).
pub fn capitalize_state(name: String) -> String {
    let name = name.replace(['.', '-'], "_");
    if name.is_empty() {
        return "Empty".to_string();
    }
    let spelled = match name.to_lowercase().as_str() {
        "pass" => "Pass".to_string(),
        "fail" => "Fail".to_string(),
        _ => capitalize_first(&name),
    };
    cpp_member(spelled)
}

/// A capitalised C++ member, escaped where the translation unit cannot
/// declare it.
///
/// No C++ keyword begins with a capital, but the preprocessor runs first: an
/// event `EOF` would be the enumerator `Event::(-1)`, and a state `__LINE__`
/// a number. A spelling that is a macro the C library headers define or a
/// name the standard reserves to the implementation
/// (`crate::reader_names::is_reserved_word`), or that begins with a digit (an
/// event `1x`), is written after an `m`: `mEOF`, `m__LINE__`, `m1x`. The
/// capitalised spelling begins with a capital, a digit or `_` and never with
/// a lower-case letter, so the escape is never the spelling of another name.
fn cpp_member(spelled: String) -> String {
    let leading_digit = spelled.starts_with(|c: char| c.is_ascii_digit());
    if leading_digit
        || crate::reader_names::is_reserved_word(crate::generator::Language::Cpp, &spelled)
    {
        format!("m{spelled}")
    } else {
        spelled
    }
}

/// An SCXML event name as a member of the C++ `Event` enum — spelled as a
/// state is. A named filter rather than the chain each template used to
/// spell, because [`crate::member_names`] has to ask exactly this question
/// of every name before the document is accepted.
pub fn to_cpp_event_variant(name: Cow<'_, str>) -> String {
    capitalize_state(name.into_owned())
}

/// Escape the author's text for the body of a C++ string literal: C's escaper,
/// since the two languages read one grammar for everything it writes.
pub(crate) fn escape_cpp(text: String) -> String {
    escape_c(text)
}

/// Escape the author's text for a C++ **format** string — `SCE_LOG_*`, which
/// expands to `fmt`.
///
/// `escape_cpp` is not enough there, and neither half of the difference is
/// theoretical. Measured 2026-08-29 on `typeof JSON.parse('{"a":1}')`:
///
/// * The `"` closed the literal early and the file did not compile —
///   `error: expected ')' before 'a'`, reported against the SCXML line by the
///   `#line` map, which is what `escape_cpp` fixes.
/// * The `{` and `}` are a `fmt` replacement field. In a build with logging
///   compiled out the macro forwards to a variadic sink and nothing parses
///   them, so the defect is invisible exactly where most gates run; with
///   logging on, `fmt` reads `{"a":1}` as a format specifier and the same
///   artifact fails at the format-string check instead.
///
/// So a format string needs both, and doubling has to happen AFTER the C++
/// escaping — `escape_c` inserts no braces, so the order is safe either way
/// today, and stating it keeps a future escape that does insert one from
/// silently un-doubling.
fn escape_cpp_format(text: String) -> String {
    escape_cpp(text).replace('{', "{{").replace('}', "}}")
}

// ── C11 filters ─────────────────────────────────────────────────

/// Register all C11-specific filters on the minijinja environment.
/// RFC §synth-5-J-1 (downstream consumer / MCU AOT backend).
///
/// `escape_c` is the C family's one string-literal escaper, which C++'s
/// `escape_cpp` forwards to: C11 and C++ read one escape grammar for
/// everything it writes. The Lua expression family is registered
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
    register_event_wire_filters(env);
    env.add_filter("escape_c", escape_c);
    // docs/adr/0086, section 8.6 (b): every reason a raise site emits comes
    // from one table, by the name of its row, and is measured for the slot.
    env.add_function("reason", crate::forge::c11_reasons::template_function);
    register_literal_door(env, crate::generator::Language::C11);
    register_ecmascript_filters(env, scope);
    env.add_filter("to_in_predicate_c11", to_in_predicate_c11);
    env.add_filter("to_c11_upper_ident", to_c11_upper_ident);
    env.add_filter("to_c11_lower_ident", to_c11_lower_ident);
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

/// Escape the author's text for the body of a C or C++ string literal.
///
/// A backslash and a double quote are escaped, and a line break, a carriage
/// return and a tab take their named escapes. Every other control character,
/// and every byte of a character past ASCII, is a three-digit octal escape:
/// unlike `\x`, which reads every hexadecimal digit after it, an octal escape
/// ends after three digits, so the character after it is never read into it;
/// and a byte written as a number is the byte whatever execution character set
/// the compiler assumes, as `c_text_initializer` writes the bytes of a text
/// constant. A `?` after a `?` is `\?`, so no pair of them begins a trigraph,
/// which ISO C11 still reads (5.2.1.1). The value as compiled is the UTF-8
/// bytes of `text`.
///
/// A U+0000 is written `\000`, and a C string stops at it: what holds the
/// literal as a NUL-terminated string holds the text before it. The expression
/// emitter refuses such a literal on C11, and C++ holds it in a `std::string`
/// of its length (`crate::forge::expr`).
pub fn escape_c(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut after_question = false;
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '?' if after_question => out.push_str("\\?"),
            ' '..='~' => out.push(ch),
            _ => {
                let mut utf8 = [0u8; 4];
                for byte in ch.encode_utf8(&mut utf8).bytes() {
                    out.push_str(&format!("\\{byte:03o}"));
                }
            }
        }
        after_question = ch == '?';
    }
    out
}

/// The most bytes of text one C string literal holds where this project's C11
/// is compiled. ISO C11 (5.2.4.1) promises 4095 characters in a string literal,
/// counted after adjacent literals are joined, and GCC's `-Woverlength-strings`,
/// which `-Wpedantic` turns on and `-Werror` makes fatal, refuses a longer one,
/// so splitting a literal into adjacent pieces does not help.
pub const C_STRING_LITERAL_MAX_BYTES: usize = 4095;

/// The most bytes of text one C++ string literal holds on the compilers this
/// project's C++ meets: MSVC holds none past 65,535 bytes, and Clang's
/// `-Woverlength-strings` refuses one past 65,536 characters.
pub const CPP_STRING_LITERAL_MAX_BYTES: usize = 65_535;

/// Each byte of `text` as a C or C++ character constant: a printable one as
/// itself, a quote and a backslash escaped, and every other byte in hex.
///
/// A list of these initializes a `char` array of any length, which no string
/// literal of either language holds; `c_text_expression`, `cpp_text_expression`
/// and the C and C++ text constants a forge unit declares write one.
pub fn c_char_constants(text: &str) -> Vec<String> {
    text.bytes()
        .map(|byte| match byte {
            b'\'' => "'\\''".to_string(),
            b'\\' => "'\\\\'".to_string(),
            0x20..=0x7e => format!("'{}'", char::from(byte)),
            other => format!("'\\x{other:02x}'"),
        })
        .collect()
}

/// `text` as a C11 expression of type `const char *`: a string literal while
/// the text fits one ([`C_STRING_LITERAL_MAX_BYTES`]), and past that a compound
/// literal of its bytes and the zero that ends them, `((const char[]){…})`,
/// which ISO C11 (6.5.2.5) holds to no length and which decays to the same
/// pointer. The outer parentheses are what let it stand as one argument of a
/// macro (`luaL_dostring` is one): the preprocessor splits a macro's arguments
/// at every comma not inside parentheses, and braces do not count. A compound
/// literal lives as long as the block it is written in, so a pointer kept past
/// the statement takes an array of static storage instead
/// ([`c_char_array_init`]). The caller refuses a text holding U+0000, which a C
/// string cannot hold.
pub fn c_text_expression(text: &str) -> String {
    if text.len() <= C_STRING_LITERAL_MAX_BYTES {
        return format!("\"{}\"", escape_c(text.to_string()));
    }
    let mut constants = c_char_constants(text);
    constants.push("'\\0'".to_string());
    format!("((const char[]){{{}}})", constants.join(", "))
}

/// `text` as a C++ expression: a string literal while the text fits one
/// ([`CPP_STRING_LITERAL_MAX_BYTES`]) and holds no U+0000, and otherwise the
/// `std::string` its bytes initialize, `(std::string{…})`, which holds a zero
/// byte as any other and is held to no length. A string literal is a
/// `const char *` that a `std::string` is made from up to its first zero, so a
/// literal holding one would lose the text after it. Parenthesized so it stands
/// as one argument of a macro, as [`c_text_expression`]'s compound literal is.
pub fn cpp_text_expression(text: &str) -> String {
    if text.len() <= CPP_STRING_LITERAL_MAX_BYTES && !text.contains('\0') {
        return format!("\"{}\"", escape_cpp(text.to_string()));
    }
    format!("(std::string{{{}}})", c_char_constants(text).join(", "))
}

/// `text` as the initializer of a C `char` array (`static const char x[] =
/// …;`): the string literal while the text fits one
/// ([`C_STRING_LITERAL_MAX_BYTES`]), and past that the brace-enclosed character
/// constants of its bytes and the zero that ends them, which give the array the
/// same bytes and the same `sizeof`. The template filter a C site that holds a
/// document text in an array writes it through.
pub fn c_char_array_init(text: String) -> String {
    if text.len() <= C_STRING_LITERAL_MAX_BYTES {
        return format!("\"{}\"", escape_c(text));
    }
    let mut constants = c_char_constants(&text);
    constants.push("'\\0'".to_string());
    format!("{{{}}}", constants.join(", "))
}

/// [`c_text_expression`] as a template filter, for a C site that hands a
/// document text to a call as a `const char *` the callee reads before it
/// returns. A pointer that is kept past the statement is given an array of
/// static storage instead ([`c_char_array_init`]): a compound literal lives
/// only as long as the block it is written in.
fn c_text_expr(text: String) -> String {
    c_text_expression(&text)
}

/// Register the literal door of `language`, the filters a C or C++ template
/// writes a document's text through ([`c_text_expr`], [`c_char_array_init`],
/// [`cpp_text_expr`]): one registration for a statechart's environment and a
/// forge kind's (a text template's literal parts), so the two hold a text alike.
/// Any other language has no door to register.
pub fn register_literal_door(
    env: &mut minijinja::Environment,
    language: crate::generator::Language,
) {
    use crate::generator::Language;
    match language {
        Language::Cpp => env.add_filter("cpp_text_expr", cpp_text_expr),
        Language::C11 => {
            env.add_filter("c_text_expr", c_text_expr);
            env.add_filter("c_char_array_init", c_char_array_init);
        }
        Language::Rust | Language::Go | Language::Kotlin | Language::Python | Language::Swift => {}
    }
}

/// [`cpp_text_expression`] as a template filter, for a C++ site that makes a
/// `std::string` of a document text.
fn cpp_text_expr(text: String) -> String {
    cpp_text_expression(&text)
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
/// The split is on blank lines that are OUTSIDE a function body, and the
/// second half of that sentence is load-bearing. Each chunk is emitted as its
/// own `luaL_dostring` whose result the generated code discards, so a chunk
/// that failed to compile would leave its definitions simply absent and the
/// first document to call one would fail at runtime with no reference back to
/// here.
///
/// ⚠ This used to break at EVERY blank line, on the stated ground that "every
/// definition in the shared assets is a top-level `function` separated from
/// its neighbours by one, so a boundary never falls inside a body". That was
/// not true: `ecma_semantics.lua` carries 25 blank lines inside bodies
/// (measured 2026-08-18), so the property held only while the running byte
/// count happened not to reach `MAX_CHUNK` at one of them. Adding a paragraph
/// anywhere earlier in either file could move a boundary into a function and
/// delete it, silently. It cost a debugging cycle when a new parser in
/// `json_builtins.lua` was split that way and surfaced as nine unrelated C11
/// failures.
///
/// Tracking depth makes the claim true by construction instead of by luck, and
/// costs nothing: a chunk simply grows past `MAX_CHUNK` until the function it
/// is inside ends.
///
/// The lines that hold only a comment are left out of every chunk
/// (`without_comment_lines`): a definition can grow past the literal floor on
/// its explanation alone, and the explanation is read in the `.lua` file, not
/// in the generated C.
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
    let mut depth: usize = 0;
    for paragraph in text.split("\n\n") {
        let paragraph = without_comment_lines(paragraph);
        if paragraph.trim().is_empty() {
            continue;
        }
        // `depth == 0` is the whole guard: a boundary is legal only between
        // definitions. The shared assets write every `function` at column 0
        // and close it with an `end` at column 0, which is what makes this
        // countable without a Lua parser — and
        // `the_chunker_tracks_function_depth_in_both_shared_assets` asserts
        // the count returns to zero for each file, so a style that broke the
        // convention would be a red rather than a silent miscount.
        if depth == 0 && !current.is_empty() && current.len() + paragraph.len() > MAX_CHUNK {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(&paragraph);
        depth = lua_depth_after(&paragraph, depth);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// `paragraph` without the lines that hold only a comment.
///
/// The shared assets carry their reasoning in comments, which a chunk does not
/// need when it runs and which count against the literal's 4095 like code
/// does. Measured 2026-11-08: the block of `ecma_semantics.lua` that binds the
/// system variables read-only is one paragraph of about 4,860 bytes, three
/// fifths of them comment, so with its comments it was a literal no
/// conforming compiler need accept, and without them it is under 2,000.
///
/// A paragraph holding a long bracket (`[[`, `[=`) is kept whole: a line of a
/// long string that begins with `--` is text rather than a comment, and the
/// line that opens a long comment cannot be dropped alone without turning the
/// comment's body into code. The shared assets hold neither today, so this
/// only keeps the rule from being wrong for a file that one day does.
fn without_comment_lines(paragraph: &str) -> String {
    if paragraph.contains("[[") || paragraph.contains("[=") {
        return paragraph.to_string();
    }
    paragraph
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Function nesting depth after reading `paragraph`, starting from `depth`.
///
/// Counts only the column-0 spellings the shared Lua assets use, because that
/// is the property being relied on rather than a general Lua parse: a
/// `function` or a `do` at column 0 opens a definition (a `do` block holds
/// file-local helpers for the definitions inside it) and an `end` at column 0
/// closes it. Anything indented belongs to a body and cannot be a boundary
/// anyway.
fn lua_depth_after(paragraph: &str, depth: usize) -> usize {
    let mut depth = depth;
    for line in paragraph.lines() {
        if line.starts_with("function ") || line.starts_with("local function ") {
            // A one-liner (`function f(a) return a end`) opens and closes on
            // the same line and must not raise the depth — `ecma_semantics.lua`
            // writes four of them, and counting them as open left the guard
            // believing it was inside a body for the rest of the file.
            if !line.ends_with(" end") {
                depth += 1;
            }
        } else if line == "do" {
            depth += 1;
        } else if line == "end" {
            depth = depth.saturating_sub(1);
        }
    }
    depth
}

fn c_lua_dostring_chunks(text: &str) -> String {
    c11_lua_chunks(text)
        .iter()
        .map(|chunk| {
            let literal = chunk
                .lines()
                .map(|line| format!("\"{}\\n\"", escape_c(line.to_string())))
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

/// Escape characters for embedding inside a JSON string literal (RFC 8259 §7),
/// in the one form every engine writes a string in (ARCHITECTURE.md, "JSON
/// Text (Single Source of Truth)"): `"` and `\` escaped, the five short forms
/// `\b \f \n \r \t`, every other U+0000-U+001F as `\u00xx` in lowercase hex,
/// everything else as it is. Only the inner escapes are produced — surrounding
/// `"..."` quotes are added by the template. Composed with `escape_c` when
/// emitted into a C string literal so the runtime bytes match cpp's
/// `EventDataHelper::scriptValueToJsonString` for a string ScriptValue
/// (`"text"` JSON-quoted form, mesh §9.6.2 wire-18 canonical wire).
///
/// A generation-time twin of the runtimes' escapers, not a call into one:
/// this crate does not link a runtime. tests/json_text/string_escape.json
/// holds both to the same cases.
pub fn escape_json_string(text: String) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let byte = c as u32 as usize;
                out.push_str("\\u00");
                out.push(char::from(HEX[byte >> 4]));
                out.push(char::from(HEX[byte & 0x0F]));
            }
            c => out.push(c),
        }
    }
    out
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
            let upper_state = to_c11_upper_ident(Cow::Borrowed(state_id));
            format!("{machine_name}_in_state(sm, {upper_machine}_STATE_{upper_state})")
        })
        .to_string()
}

/// An SCXML name as the part it contributes to a C11 upper-case symbol:
/// `<PREFIX><MACHINE>_STATE_<ID>`, `…_EVENT_<NAME>`, `…_HIST_<ID>`.
///
/// Upper-cased, with `.` and `-` — legal in an XML Name and an event name,
/// legal in no C identifier — written `_`. One function for every such
/// symbol rather than the filter chain the C11 templates used to repeat,
/// because [`crate::member_names`] has to ask exactly this question of every
/// name before the document is accepted: two names that spell one symbol
/// declare one enumerator twice.
pub fn to_c11_upper_ident(name: Cow<'_, str>) -> String {
    name.to_uppercase().replace(['-', '.'], "_")
}

/// The same part of a C11 lower-case symbol (a per-state function name).
/// Kept beside [`to_c11_upper_ident`] because the two fold names together
/// the same way only for ASCII: Unicode case mapping is not symmetric.
pub fn to_c11_lower_ident(name: Cow<'_, str>) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

// ── Kotlin filters ───────────────────────────────────────────────

/// Register all Kotlin-specific filters on the minijinja environment.
///
/// Carries the scope for the same single filter, and for the same reason,
/// as [`register_cpp_filters`].
pub fn register_kotlin_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    register_kotlin_filters_for_engine(
        env,
        scope,
        crate::generator::Language::Kotlin.default_script_engine_target(),
    )
}

/// [`register_kotlin_filters`] with the run's engine selection named.
///
/// Same two-function shape as the C++ side next door, and for the same
/// reason: Rust has no default arguments, and every caller that does not pass
/// a target means this backend's default.
pub fn register_kotlin_filters_for_engine(
    env: &mut minijinja::Environment,
    scope: &Arc<DocumentScope>,
    target: crate::generator::ScriptEngineTarget,
) {
    register_script_source_filters(env, scope, target, ScriptSourceSpelling::Kotlin);
    register_kotlin_filters_inner(env, scope)
}

fn register_kotlin_filters_inner(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    register_inline_content_reading(env, scope);
    let for_content = Arc::clone(scope);
    env.add_filter("to_author_data_content", move |content: String| {
        to_author_data_content(content, &for_content)
    });
    let for_assign = Arc::clone(scope);
    env.add_filter("to_author_assign_content", move |content: String| {
        to_author_assign_content(content, &for_assign)
    });
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    register_event_wire_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_camel_case", to_camel_case);
    env.add_filter("escape_kotlin", escape_kotlin);
    env.add_filter("to_kotlin_string_expr", to_kotlin_string_expr);
    env.add_filter("to_event_class_name", to_event_class_name);
    // The stem of an event's typed-payload field, `pending<Variant>Payload` —
    // the one spelling the payload channel declares it by.
    env.add_filter("to_event_variant", to_event_variant);
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

/// Escape the author's text for the body of a Kotlin `"…"` string literal.
///
/// A backslash and a double quote are escaped, and so is every `$`, which
/// would otherwise open a template expression (`$name`, `${…}`) and run the
/// author's text as code. A line break, a carriage return and a tab take
/// their named escapes, and every other control character, and each character
/// that changes the direction text is shown in, is written `\uXXXX`.
pub(crate) fn escape_kotlin(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || changes_text_direction(c) => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)))
            }
            c => out.push(c),
        }
    }
    out
}

/// The most bytes one string constant of a JVM class file holds: a
/// `CONSTANT_Utf8_info` counts its bytes, in modified UTF-8, in a `u2` (The
/// Java Virtual Machine Specification, section 4.4.7).
pub const JVM_CONSTANT_MAX_BYTES: usize = 65_535;

/// The bytes `c` takes in modified UTF-8 (JVM Specification, section 4.4.7):
/// U+0000 two, U+0001 to U+007F one, to U+07FF two, to U+FFFF three, and a
/// character past it six, the two surrogates of its UTF-16 pair three each.
pub fn modified_utf8_len(c: char) -> usize {
    match u32::from(c) {
        0 => 2,
        0x01..=0x7f => 1,
        0x80..=0x7ff => 2,
        0x800..=0xffff => 3,
        _ => 6,
    }
}

/// `text` cut into pieces each of at most [`JVM_CONSTANT_MAX_BYTES`] bytes of
/// modified UTF-8, at character boundaries, so a character is never split;
/// one piece, the text itself, when it fits.
pub fn jvm_constant_chunks(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut bytes = 0;
    for (at, c) in text.char_indices() {
        let width = modified_utf8_len(c);
        if bytes + width > JVM_CONSTANT_MAX_BYTES {
            chunks.push(&text[start..at]);
            start = at;
            bytes = 0;
        }
        bytes += width;
    }
    chunks.push(&text[start..]);
    chunks
}

/// The Kotlin declaration of the text constant `name`, `text` its value:
/// `const val NAME: String = "…"` while the text fits one class-file constant,
/// and otherwise a `val` built when the file's class is initialized from pieces
/// that each fit ([`jvm_constant_chunks`]), since kotlinc cannot write a
/// `const val` past [`JVM_CONSTANT_MAX_BYTES`]. `indent` is the indentation the
/// declaration stands at, which the lines of the pieces take too.
///
/// The one door a Kotlin template writes a generated text constant through,
/// whatever its length, so a text no generator bounds (a payload's JSON
/// Schema, an event name) compiles at every length. A constant of a fixed
/// vocabulary (a hash, a progress word) needs none; the census
/// `sce-build/tests/a_kotlin_class_file_constant_takes_its_text_through_one_door.rs`
/// refuses any other text constant, and lists those with the reason for each.
///
/// # Why a constant, and not every literal
///
/// kotlinc cuts a long string itself wherever the string is an expression,
/// so only the places where it writes the string into a class-file attribute
/// need this door. Read from the compiler the build pins (kotlinc 2.1.20,
/// `gradle/libs.versions.toml`; `javap -c` over
/// `kotlin-compiler-embeddable-2.1.20.jar`), and the same in 2.0.20:
///
/// - `org.jetbrains.kotlin.backend.jvm.codegen.ExpressionCodegen.visitConst`
///   hands every `String` constant to `generateStringConstant`, which cuts it
///   with `org.jetbrains.kotlin.codegen.CodegenUtilKt.splitStringConstant`
///   (modified UTF-8 counted per UTF-16 unit against 65,535, the count of
///   [`modified_utf8_len`]) and, past one piece, builds the string with one
///   `StringBuilder.append` per piece. A literal in a function body or in a
///   property initializer therefore compiles at any length.
/// - `org.jetbrains.kotlin.codegen.StringConcatGenerator.fitRestrictions`
///   does the same for the constant parts of a concatenation or a string
///   template (`"a" + x`, `"a$x"`), and moves the largest of them out of an
///   `invokedynamic` recipe whose bytes would pass 65,535.
/// - kotlinc folds `"a" + "b"` into one constant (a `const val` may be
///   initialized so), and in an expression the folded constant is cut like
///   any other, so folding changes nothing there.
///
/// What kotlinc does not cut is a constant it writes into an attribute: the
/// `ConstantValue` attribute of a `const val`'s field (written for a
/// `const val` only, never for a plain `val`) and an annotation's argument.
/// A text past [`JVM_CONSTANT_MAX_BYTES`] in either does not compile, so a
/// text no generator bounds reaches neither except through this function.
pub fn kotlin_string_val(name: String, text: String, indent: Option<String>) -> String {
    let chunks = jvm_constant_chunks(&text);
    if let [whole] = chunks.as_slice() {
        return format!(
            "const val {name}: String = \"{}\"",
            escape_kotlin(whole.to_string())
        );
    }
    let indent = indent.unwrap_or_default();
    let mut out = format!("val {name}: String = buildString {{\n");
    for chunk in chunks {
        out.push_str(&format!(
            "{indent}    append(\"{}\")\n",
            escape_kotlin(chunk.to_string())
        ));
    }
    out.push_str(&indent);
    out.push('}');
    out
}

/// `text` as a Kotlin expression of type `String`: a literal while it fits one
/// class-file constant, and otherwise the `buildString` of pieces that each do
/// ([`jvm_constant_chunks`]), which kotlinc does not fold back into one
/// constant. The expression twin of [`kotlin_string_val`].
pub fn kotlin_text_expression(text: &str) -> String {
    let chunks = jvm_constant_chunks(text);
    if let [whole] = chunks.as_slice() {
        return format!("\"{}\"", escape_kotlin(whole.to_string()));
    }
    let appends: Vec<String> = chunks
        .iter()
        .map(|chunk| format!("append(\"{}\")", escape_kotlin(chunk.to_string())))
        .collect();
    format!("buildString {{ {} }}", appends.join("; "))
}

/// Escape the author's text for the BODY of a Swift string literal.
///
/// Both forms Swift writes with escapes — `"…"` and `"""…"""` — read the same
/// ones, so one escaper serves both; the `#`-delimited raw forms read none and
/// are written by `crate::literal_text`'s raw rule instead. The backslash is
/// what opens an escape and, before `(`, an interpolation, so doubling it is
/// what keeps a value from running as code. `"` is escaped for the single-line
/// form (it is harmless in the multi-line one), and every other control
/// character is written `\u{…}`, which is the one numeric form Swift has.
pub(crate) fn escape_swift(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

/// A character as TTCN-3 writes it by its position in ISO/IEC 10646:
/// `char(group, plane, row, cell)`, a value of `universal charstring`
/// (ES 201 873-1 §6.1.1).
fn ttcn3_quadruple(ch: char) -> String {
    let point = u32::from(ch);
    format!(
        "char({}, {}, {}, {})",
        point >> 24,
        (point >> 16) & 0xFF,
        (point >> 8) & 0xFF,
        point & 0xFF
    )
}

/// Escape the author's text for the BODY of a TTCN-3 character string
/// literal, `"…"`.
///
/// TTCN-3 writes a `"` inside the literal twice and reads a backslash as
/// itself. It has no escape for any other character, so one outside printable
/// ASCII ends the literal and is concatenated as its quadruple
/// ([`ttcn3_quadruple`]): what the escaper writes between the literal's quotes
/// is `" & char(0, 0, 0, 10) & "`, which with them is an expression of the
/// same characters. A module the export writes is ASCII whatever its values
/// hold.
pub(crate) fn escape_ttcn3(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\"\""),
            ' '..='~' => out.push(ch),
            other => {
                out.push_str("\" & ");
                out.push_str(&ttcn3_quadruple(other));
                out.push_str(" & \"");
            }
        }
    }
    out
}

/// `text` as a TTCN-3 expression of type `universal charstring`, its
/// delimiters included: each run of printable ASCII one literal, written as
/// [`escape_ttcn3`] writes it, and each other character its quadruple,
/// joined by `&`; `""` for no text. The one spelling the TTCN-3 export
/// writes a text value by (docs/SCE_TEST_PROFILE.md §9.5).
pub(crate) fn ttcn3_charstring(text: String) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut run = String::new();
    for ch in text.chars() {
        match ch {
            ' '..='~' => run.push(ch),
            other => {
                if !run.is_empty() {
                    parts.push(format!("\"{}\"", escape_ttcn3(std::mem::take(&mut run))));
                }
                parts.push(ttcn3_quadruple(other));
            }
        }
    }
    if !run.is_empty() || parts.is_empty() {
        parts.push(format!("\"{}\"", escape_ttcn3(run)));
    }
    parts.join(" & ")
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
        .map(to_kotlin_event_segment)
        .collect::<Vec<_>>()
        .join(".")
}

/// One dot-separated token of an event name (§scxml-3.12.1) as the Kotlin
/// class it becomes: `door.open` is `Door.Open`, one nested type per token.
///
/// The event tree ([`crate::kotlin::render_event_tree`]) declares these
/// classes and [`to_event_class_name`] refers to them, so both spell a token
/// here — two spellings would declare one class and name another.
///
/// A token may begin with a digit (`DIGIT.0`) or be delimiters alone (`_`),
/// and a class name may be neither, so the spelling is made an identifier as
/// [`joined_identifier`] says: `DIGIT.0` is `DIGIT._0`, a token `_` is
/// `_Blank`. No Kotlin keyword begins with a capital, so no token needs a
/// keyword escape.
pub fn to_kotlin_event_segment(segment: &str) -> String {
    let joined = RE_WORD_DELIMITERS
        .split(segment)
        .map(|p| {
            if p.is_empty() {
                String::new()
            } else {
                capitalize_first(p)
            }
        })
        .collect::<String>();
    joined_identifier(joined, "Blank")
}

/// Convert SCXML state ID to Kotlin PascalCase class name: the spelling of
/// [`to_state_variant`], keyword escape included.
pub fn to_state_class_name(name: String) -> String {
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
/// No keyword filter is registered here: state and event identifiers are
/// UPPER_SNAKE_CASE (IntEnum members), which no keyword is. The one place an
/// author's identifier does reach a Python parser — a typed datamodel
/// reader's method name — is spelled by [`crate::reader_names`], which
/// escapes a keyword and refuses a collision for every backend at once.
pub fn register_python_filters(env: &mut minijinja::Environment, scope: &Arc<DocumentScope>) {
    env.add_filter("w3c_session_name", w3c_session_name);
    register_invoke_filters(env);
    register_event_wire_filters(env);
    env.add_filter("to_pascal_case", to_pascal_case);
    env.add_filter("to_snake_case", to_snake_case);
    env.add_filter("to_python_const", to_python_const);
    env.add_filter("py_string_literal", py_string_literal);
    env.add_filter("py_json_literal", py_json_literal);
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
    // §scxml-5.3.1: `<data src="file:...">` inlining at codegen time —
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
/// A name that starts with a digit gets an underscore prefix so the resulting
/// member is a valid identifier. No keyword of Python is written in capitals,
/// and of its soft keywords only `_` can be spelled, which names a member as
/// any name does. A spelling `Enum` reads as no member is escaped as
/// [`python_enum_member`] says.
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
    python_enum_member(prefixed)
}

/// An `UPPER_SNAKE` spelling as an ordinary member of a Python `Enum`.
///
/// `Enum` reads three forms of name in a class body as something other than
/// a member: a `_sunder_` name is refused unless it is one `Enum` gives a
/// meaning (`_order_`), a `__dunder__` name is an attribute, and a `__private`
/// name is mangled to `_<Class>__private` and then left out. A state `_x_`
/// is spelled the first (`_X_`), and so is an event `1x_` once the leading
/// digit is written after a `_` (`_1X_`). Each such spelling, and every one
/// that begins `__`, is written after an `m`: `m_X_`, `m__X`. The
/// `UPPER_SNAKE` spelling holds no lower-case letter, so the
/// escape is never the spelling of another name. The lower-case name `Enum`
/// refuses as a member, `mro`, is none it can spell.
fn python_enum_member(spelled: String) -> String {
    let sunder_or_dunder = spelled.len() > 2 && spelled.starts_with('_') && spelled.ends_with('_');
    if sunder_or_dunder || spelled.starts_with("__") {
        format!("m{spelled}")
    } else {
        spelled
    }
}

/// Escape the author's text for the BODY of a Python string literal.
///
/// The sibling of `escape_rust` / `escape_go` / `escape_kotlin`, and the one
/// the family was missing: `py_string_literal` below returns a COMPLETE
/// literal, quotes included, so it cannot be used at a site that already sits
/// inside quotes — writing it there would close the literal and open another.
/// `crate::literal_text` needs a body escaper for every syntax it routes, and
/// this is Python's.
///
/// The grammar is `py_string_literal`'s own, minus the delimiters, so the two
/// agree on what a Python literal may carry.
pub fn escape_python(text: String) -> String {
    let mut out = String::with_capacity(text.len());
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
    out
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

/// The Python value a JSON text denotes, as Python source: `None`, `True`,
/// `False`, the number as JSON spells it, a string as [`py_string_literal`]
/// writes it, and a list or a dict of those.
///
/// For a `<send>` whose inline `<content>` is a value fixed at build time
/// ([`InlineContentReading::Value`]): the payload `_data` holds is the value
/// itself, the thing an evaluation would have handed back, so every path that
/// reads `_data` reads it the same way. A text that is not JSON is a defect of
/// the caller, refused rather than written as something else.
pub fn py_json_literal(json: String) -> Result<String, minijinja::Error> {
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("py_json_literal: `{json}` is not JSON: {e}"),
        )
    })?;
    Ok(py_value_literal(&value))
}

fn py_value_literal(value: &serde_json::Value) -> String {
    use serde_json::Value as J;
    match value {
        J::Null => "None".to_string(),
        J::Bool(true) => "True".to_string(),
        J::Bool(false) => "False".to_string(),
        J::Number(n) => n.to_string(),
        J::String(s) => py_string_literal(s.clone()),
        J::Array(items) => {
            let parts: Vec<String> = items.iter().map(py_value_literal).collect();
            format!("[{}]", parts.join(", "))
        }
        J::Object(members) => {
            let parts: Vec<String> = members
                .iter()
                .map(|(key, item)| {
                    format!(
                        "{}: {}",
                        py_string_literal(key.clone()),
                        py_value_literal(item)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// A Kotlin text constant past one class-file constant is built at class init
/// from pieces that each fit, counted in modified UTF-8.
#[cfg(test)]
mod kotlin_text_constants {
    use super::*;

    /// The bytes of modified UTF-8 `text` takes.
    fn modified(text: &str) -> usize {
        text.chars().map(modified_utf8_len).sum()
    }

    #[test]
    fn a_character_takes_its_bytes_of_modified_utf8() {
        assert_eq!(modified_utf8_len('\u{0}'), 2);
        assert_eq!(modified_utf8_len('a'), 1);
        assert_eq!(modified_utf8_len('\u{e9}'), 2);
        assert_eq!(modified_utf8_len('\u{20ac}'), 3);
        assert_eq!(modified_utf8_len('\u{1f600}'), 6);
    }

    /// At 65,535 bytes the text is one constant; at 65,536 it is two pieces,
    /// the first full, whatever the widths of its characters, and a character
    /// that would cross the limit opens the next piece whole.
    #[test]
    fn a_text_past_one_constant_is_cut_at_the_limit() {
        let euro = "\u{20ac}";
        for (fits, past) in [
            ("a".repeat(65_535), "a".repeat(65_536)),
            (euro.repeat(21_845), format!("{}a", euro.repeat(21_845))),
            ("\u{0}".repeat(32_767) + "a", "\u{0}".repeat(32_768)),
        ] {
            assert_eq!(modified(&fits), JVM_CONSTANT_MAX_BYTES);
            assert_eq!(jvm_constant_chunks(&fits), [fits.as_str()]);
            assert_eq!(modified(&past), JVM_CONSTANT_MAX_BYTES + 1);
            let chunks = jvm_constant_chunks(&past);
            assert_eq!(chunks.len(), 2);
            assert_eq!(chunks.concat(), past);
            assert!(chunks.iter().all(|c| modified(c) <= JVM_CONSTANT_MAX_BYTES));
        }
        // Six bytes that would cross the limit start the second piece.
        let crossing = format!("{}\u{1f600}", "a".repeat(65_530));
        let chunks = jvm_constant_chunks(&crossing);
        let ascii = "a".repeat(65_530);
        assert_eq!(chunks, [ascii.as_str(), "\u{1f600}"]);
        assert_eq!(jvm_constant_chunks(""), [""]);
    }

    #[test]
    fn a_kotlin_constant_past_the_limit_is_built_at_class_init() {
        assert_eq!(
            kotlin_string_val("NAME".into(), "a$\"b".into(), None),
            "const val NAME: String = \"a\\$\\\"b\""
        );
        let fits = kotlin_string_val("NAME".into(), "a".repeat(65_535), None);
        assert!(
            fits.starts_with("const val NAME: String = \"aaa"),
            "{}",
            &fits[..40]
        );
        assert_eq!(fits.lines().count(), 1);
        let past = kotlin_string_val(
            "NAME".into(),
            format!("{}$", "a".repeat(65_535)),
            Some("    ".into()),
        );
        let lines: Vec<&str> = past.lines().collect();
        assert_eq!(lines.len(), 4, "a head, two pieces and the close");
        assert_eq!(lines[0], "val NAME: String = buildString {");
        assert_eq!(
            lines[1],
            format!("        append(\"{}\")", "a".repeat(65_535))
        );
        assert_eq!(lines[2], "        append(\"\\$\")");
        assert_eq!(lines[3], "    }");
    }

    /// The limit is the constant's and not its spelling's: a character the
    /// literal escapes counts the one byte it stands for, so 65,535 backslashes
    /// are one constant though the source spells them in twice as many.
    #[test]
    fn a_kotlin_constant_is_measured_as_the_text_and_not_its_escapes() {
        let backslashes = "\\".repeat(65_535);
        assert_eq!(
            kotlin_string_val("NAME".into(), backslashes.clone(), None),
            format!("const val NAME: String = \"{}\"", "\\\\".repeat(65_535))
        );
        let past = kotlin_string_val("NAME".into(), format!("{backslashes}\\"), None);
        let lines: Vec<&str> = past.lines().collect();
        assert_eq!(lines.len(), 4, "a head, two pieces and the close");
        assert_eq!(lines[0], "val NAME: String = buildString {");
        assert_eq!(
            lines[1],
            format!("    append(\"{}\")", "\\\\".repeat(65_535))
        );
        assert_eq!(lines[2], "    append(\"\\\\\")");
        assert_eq!(lines[3], "}");
    }
}

#[cfg(test)]
mod literal_escaper_census {
    use super::*;
    use crate::literal_text::ALREADY_FIT_FOR_A_LITERAL;
    use std::collections::BTreeSet;

    /// A filter that takes the author's text and returns what a template
    /// writes in its place.
    type TextFilter = fn(String) -> String;

    /// Every filter this module registers whose name begins `escape_`, paired
    /// with the function it registers.
    ///
    /// The pairing is what lets the census below ask each one what it DOES.
    /// `every_registered_escaper_appears_here` proves the table is complete,
    /// so this is an index of the registrations rather than a hand-picked
    /// list that can quietly fall behind them.
    const ESCAPERS: &[(&str, TextFilter)] = &[
        ("escape_c", escape_c),
        ("escape_cpp", escape_cpp),
        ("escape_cpp_format", escape_cpp_format),
        ("escape_go", escape_go),
        ("escape_go_keyword", escape_go_keyword),
        ("escape_json_string", escape_json_string),
        ("escape_kotlin", escape_kotlin),
        ("escape_lua", escape_lua),
        ("escape_python", escape_python),
        ("escape_rust", escape_rust),
        ("escape_swift", escape_swift),
        ("escape_ttcn3", escape_ttcn3),
    ];

    /// Text that a STRING-LITERAL escaper must change and an identifier
    /// renamer must not: it carries the two characters that end a literal
    /// early, and it is not any language's keyword.
    const HOSTILE: &str = "a\"b\\c";

    /// A filter belongs in `ALREADY_FIT_FOR_A_LITERAL` exactly when it
    /// escapes for a literal — and what decides that is what it does, not
    /// what it is called.
    ///
    /// ⚠ The name cannot decide it, and this test exists because relying on
    /// the name got it wrong: `escape_go_keyword` begins with the same six
    /// letters as the real escapers and is nothing like them — it renames an
    /// identifier that collides with a Go keyword (`type` to `type_`) and
    /// leaves every other string alone. Declaring one fit for a literal would tell the
    /// door a value had been escaped when nothing had touched it.
    ///
    /// Delegation is the second reason a source scan cannot answer it:
    /// `escape_cpp` is one line, forwarding to `escape_c`, so its own body
    /// never mentions a quote.
    #[test]
    fn a_filter_is_declared_fit_exactly_when_it_escapes_for_a_literal() {
        let declared: BTreeSet<&str> = ALREADY_FIT_FOR_A_LITERAL.iter().copied().collect();
        for (name, f) in ESCAPERS {
            let escapes = f(HOSTILE.to_string()) != HOSTILE;
            assert_eq!(
                escapes,
                declared.contains(name),
                "{name} {} the hostile text but is {} \
                 ALREADY_FIT_FOR_A_LITERAL. A filter that escapes must be \
                 declared or the door escapes it twice; one that does not \
                 must not be, or the door believes a value is safe when \
                 nothing has touched it.",
                if escapes { "changes" } else { "leaves" },
                if declared.contains(name) {
                    "in"
                } else {
                    "not in"
                }
            );
        }
    }

    /// What `escape_swift` writes is what a Swift compiler reads back as the
    /// value, in both literal forms that process escapes. The expected text
    /// is the one compiled with Swift 6.4.0 as `"…"` and as `"""…"""` and
    /// compared byte for byte with the original.
    #[test]
    fn escape_swift_writes_what_swift_reads_back() {
        let value = "a\"b\\c\n\r\t\0\u{1}\u{7f}(x)\\(y) \u{1F600} é";
        assert_eq!(
            escape_swift(value.to_string()),
            "a\\\"b\\\\c\\n\\r\\t\\0\\u{1}\\u{7f}(x)\\\\(y) \u{1F600} é"
        );
        // A backslash before `(` is doubled, so nothing interpolates.
        assert_eq!(escape_swift("\\(1)".to_string()), "\\\\(1)");
    }

    /// What `escape_ttcn3` writes between a literal's quotes is an expression
    /// of the same characters: a `"` twice, a backslash as itself, and every
    /// character outside printable ASCII concatenated as its quadruple.
    #[test]
    fn escape_ttcn3_writes_what_ttcn3_reads_back() {
        assert_eq!(escape_ttcn3("a\"b\\c".to_string()), "a\"\"b\\c");
        assert_eq!(
            escape_ttcn3("x\ny".to_string()),
            "x\" & char(0, 0, 0, 10) & \"y"
        );
        assert_eq!(
            escape_ttcn3("\u{e9}\u{1F600}".to_string()),
            "\" & char(0, 0, 0, 233) & \"\" & char(0, 1, 246, 0) & \""
        );
    }

    /// `ttcn3_charstring` writes a whole expression: one literal per run of
    /// printable ASCII, a quadruple per other character, and `""` for none.
    #[test]
    fn ttcn3_charstring_writes_a_whole_expression() {
        assert_eq!(ttcn3_charstring(String::new()), "\"\"");
        assert_eq!(
            ttcn3_charstring("started on time".to_string()),
            "\"started on time\""
        );
        assert_eq!(
            ttcn3_charstring("say \"hi\"\n".to_string()),
            "\"say \"\"hi\"\"\" & char(0, 0, 0, 10)"
        );
        assert_eq!(
            ttcn3_charstring("\tx".to_string()),
            "char(0, 0, 0, 9) & \"x\""
        );
    }

    /// The table above names every `escape_*` filter this module registers.
    ///
    /// Read off the registrations rather than restated, so a new escaper
    /// added to an environment fails here until it is measured.
    #[test]
    fn every_registered_escaper_appears_here() {
        let text = include_str!("filters.rs");
        let needle = "add_filter(\"";
        let mut registered: BTreeSet<&str> = BTreeSet::new();
        for (at, _) in text.match_indices(needle) {
            let rest = &text[at + needle.len()..];
            let Some(end) = rest.find('"') else { continue };
            let name = &rest[..end];
            if name.starts_with("escape_") {
                registered.insert(name);
            }
        }
        // A floor: an empty sweep would pass while measuring nothing.
        assert!(
            registered.len() >= 5,
            "only {} escape_* registration(s) found; the scan has stopped \
             reading them",
            registered.len()
        );
        let tabled: BTreeSet<&str> = ESCAPERS.iter().map(|(n, _)| *n).collect();
        let missing: Vec<&&str> = registered.difference(&tabled).collect();
        assert!(
            missing.is_empty(),
            "escape_* filter(s) registered but absent from ESCAPERS: \
             {missing:?} — add each with its function so the census can ask \
             what it does"
        );
    }
}

#[cfg(test)]
mod json_text_table {
    use super::escape_json_string;

    /// tests/json_text/string_escape.json: the cases every engine's writer is
    /// measured against. The generation-time escaper writes the text a
    /// generated `_event.data` carries, so it is held to them too.
    #[test]
    fn the_generation_time_escaper_writes_the_one_form() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/json_text/string_escape.json"
        );
        let table: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("the shared escape table"))
                .expect("the table is JSON");
        let cases = table["cases"].as_array().expect("the table has cases");
        assert!(cases.len() >= 8, "the table lost cases: {}", cases.len());
        for case in cases {
            let text = |name: &str| {
                case[name]
                    .as_str()
                    .unwrap_or_else(|| panic!("a case's {name} is a string"))
                    .to_string()
            };
            assert_eq!(
                escape_json_string(text("text")),
                text("escaped"),
                "{}",
                text("name")
            );
        }
    }
}

#[cfg(test)]
mod inline_content_readings {
    use super::InlineContentReading::{Expression, Xml};
    use super::*;
    use crate::model::Datamodel;

    fn read(content: &str, datamodel: Datamodel) -> InlineContentReading {
        inline_content_reading(content, datamodel, &DocumentScope::installed())
    }

    fn fixed(wire: &str) -> InlineContentReading {
        InlineContentReading::Value(wire.to_string())
    }

    /// Under ECMAScript a literal is the value it denotes, finished at build
    /// time as the JSON the event carries, so no engine is needed to read it.
    #[test]
    fn a_literal_under_ecmascript_is_its_value() {
        let ecma = Datamodel::EcmaScript;
        assert_eq!(read("21", ecma), fixed("21"));
        assert_eq!(read("\n  21\n", ecma), fixed("21"));
        assert_eq!(read("-1.5", ecma), fixed("-1.5"));
        assert_eq!(read("0x10", ecma), fixed("16"));
        assert_eq!(read("'it'", ecma), fixed("\"it\""));
        assert_eq!(read("true", ecma), fixed("true"));
        assert_eq!(read("null", ecma), fixed("null"));
        assert_eq!(
            read("{ \"b\": [1, true, null], \"a\": 'x\"y' }", ecma),
            fixed("{\"a\":\"x\\\"y\",\"b\":[1,true,null]}")
        );
    }

    /// An expression whose value needs the data model is evaluated when the
    /// send runs, which is what costs the machine a script engine.
    #[test]
    fn an_expression_under_ecmascript_is_evaluated() {
        let ecma = Datamodel::EcmaScript;
        assert_eq!(read("1 + 1", ecma), Expression);
        assert_eq!(read("!0", ecma), Expression);
        assert_eq!(read("[1, 1 + 1]", ecma), Expression);
        let scope = DocumentScope::declaring(["x"]);
        assert_eq!(inline_content_reading("x", ecma, &scope), Expression);
        assert!(inline_content_reading("x", ecma, &scope).is_evaluated());
        assert!(!read("21", ecma).is_evaluated());
    }

    /// Text that is not an expression of the document, a name it does not
    /// declare among them, is the space-normalized string.
    #[test]
    fn text_that_is_no_expression_is_the_normalized_string() {
        let ecma = Datamodel::EcmaScript;
        assert_eq!(read("  two   words\n", ecma), fixed("\"two words\""));
        assert_eq!(read("x + 1", ecma), fixed("\"x + 1\""));
    }

    /// XML travels as written whatever the data model: the receiver builds the
    /// DOM from the text.
    #[test]
    fn xml_travels_as_written() {
        for datamodel in Datamodel::ALL {
            assert_eq!(
                read("  <a>1</a>", datamodel),
                Xml("  <a>1</a>".to_string()),
                "{datamodel:?}"
            );
        }
    }

    /// With no value expression language to read the text in, the text is the
    /// value: the reading is the data model's, never the rest of the document's.
    #[test]
    fn without_value_expressions_the_text_is_the_value() {
        for datamodel in [Datamodel::Null, Datamodel::SceStatic] {
            assert_eq!(read("21", datamodel), fixed("\"21\""), "{datamodel:?}");
            assert_eq!(
                read(" 1  + 1 ", datamodel),
                fixed("\"1 + 1\""),
                "{datamodel:?}"
            );
            assert_eq!(
                read("21", datamodel),
                fixed(&static_content_wire("21")),
                "{datamodel:?}"
            );
        }
    }

    /// A template branches on `kind` and writes `wire`.
    #[test]
    fn a_template_reads_the_kind_and_the_wire() {
        assert_eq!(
            serde_json::to_value(fixed("21")).expect("serializes"),
            serde_json::json!({"kind": "value", "wire": "21"})
        );
        assert_eq!(
            serde_json::to_value(Xml("<a/>".to_string())).expect("serializes"),
            serde_json::json!({"kind": "xml", "wire": "<a/>"})
        );
        assert_eq!(
            serde_json::to_value(Expression).expect("serializes"),
            serde_json::json!({"kind": "expression"})
        );
    }

    /// `data_content_is_xml` is the ECMAScript reading's XML rung.
    #[test]
    fn data_content_is_xml_is_the_xml_rung() {
        let scope = DocumentScope::installed();
        assert!(data_content_is_xml("\n <books/>".to_string(), &scope));
        assert!(!data_content_is_xml("21".to_string(), &scope));
        assert!(!data_content_is_xml("two words".to_string(), &scope));
        assert!(!data_content_is_xml(String::new(), &scope));
    }

    /// tests/json_text/real_text.json: the spelling every engine's writer is
    /// held to, which a number finished at build time is written in too.
    #[test]
    fn a_number_is_spelled_as_every_engine_spells_it() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/json_text/real_text.json"
        );
        let table: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("the shared number table"))
                .expect("the table is JSON");
        let cases = table["cases"].as_array().expect("the table has cases");
        assert!(cases.len() >= 50, "the table lost cases: {}", cases.len());
        for case in cases {
            let field = |name: &str| {
                case[name]
                    .as_str()
                    .unwrap_or_else(|| panic!("a case's {name} is a string"))
                    .to_string()
            };
            let bits = u64::from_str_radix(&field("bits"), 16).expect("bits are hex");
            assert_eq!(
                json_number_text(f64::from_bits(bits)),
                field("text"),
                "{}",
                field("name")
            );
        }
    }

    /// The Python value a JSON text denotes.
    #[test]
    fn a_json_value_is_the_python_value_it_denotes() {
        let py = |json: &str| py_json_literal(json.to_string()).expect("JSON");
        assert_eq!(py("21"), "21");
        assert_eq!(py("-1.5"), "-1.5");
        assert_eq!(py("\"two words\""), "\"two words\"");
        assert_eq!(py("null"), "None");
        assert_eq!(
            py("{\"a\":[true,false],\"b\":\"x\\\"y\"}"),
            "{\"a\": [True, False], \"b\": \"x\\\"y\"}"
        );
        assert!(py_json_literal("two words".to_string()).is_err());
    }
}

/// The member a state, a history or an event becomes in each backend is an
/// identifier the backend can declare, whatever conforming name it is.
#[cfg(test)]
mod member_spelling {
    use super::*;

    #[test]
    fn a_pascal_member_escapes_a_keyword_a_leading_digit_and_a_blank() {
        assert_eq!(to_state_variant("self".to_string()), "Self_");
        assert_eq!(to_event_variant("Self".to_string()), "Self_");
        assert_eq!(to_event_variant("1x".to_string()), "_1x");
        assert_eq!(to_state_variant("_1".to_string()), "_1");
        assert_eq!(to_state_variant("_".to_string()), "_Blank");
        assert_eq!(to_state_variant("_-_".to_string()), "_Blank");
        // A name no rule touches is spelled as it always was.
        assert_eq!(to_state_variant("door-open".to_string()), "DoorOpen");
        assert_eq!(to_event_variant("DIGIT.0".to_string()), "DIGIT0");
        assert_eq!(to_state_class_name("self".to_string()), "Self_");
    }

    #[test]
    fn a_kotlin_event_token_escapes_a_leading_digit_and_a_blank() {
        assert_eq!(to_kotlin_event_segment("0"), "_0");
        assert_eq!(to_kotlin_event_segment("_"), "_Blank");
        assert_eq!(to_kotlin_event_segment("open"), "Open");
        // No Kotlin keyword is capitalised, and the `Self` an event that is
        // also a prefix declares stays a collision `member_names` refuses.
        assert_eq!(to_kotlin_event_segment("self"), "Self");
    }

    #[test]
    fn a_swift_case_escapes_what_a_backtick_does_not_reach() {
        assert_eq!(to_swift_variant("self".to_string()), "self_");
        assert_eq!(to_swift_variant("init".to_string()), "init_");
        assert_eq!(to_swift_variant("Type".to_string()), "Type_");
        assert_eq!(to_swift_variant("break".to_string()), "`break`");
        assert_eq!(to_swift_variant("1x".to_string()), "_1x");
        assert_eq!(to_swift_variant("_".to_string()), "_blank");
        assert_eq!(to_swift_variant("doorOpen".to_string()), "doorOpen");
    }

    #[test]
    fn a_cpp_member_escapes_what_the_preprocessor_would_expand() {
        for (name, spelled) in [
            ("EOF", "mEOF"),
            ("NULL", "mNULL"),
            ("INFINITY", "mINFINITY"),
            ("EXIT_SUCCESS", "mEXIT_SUCCESS"),
            ("TRUE", "mTRUE"),
            ("SIGINT", "mSIGINT"),
            ("INT32_MAX", "mINT32_MAX"),
            ("UINT_LEAST8_MAX", "mUINT_LEAST8_MAX"),
            ("DBL_EPSILON", "mDBL_EPSILON"),
            ("PRId64", "mPRId64"),
            ("__LINE__", "m__LINE__"),
            ("_Bool", "m_Bool"),
            ("1x", "m1x"),
        ] {
            assert_eq!(capitalize_state(name.to_string()), spelled, "{name}");
        }
        // The lower-case macros cannot be a capitalised member at all.
        assert_eq!(capitalize_state("errno".to_string()), "Errno");
        assert_eq!(capitalize_state("stdin".to_string()), "Stdin");
        assert_eq!(capitalize_state("assert".to_string()), "Assert");
        // A name nothing reserves, as it always was.
        for (name, spelled) in [
            ("idle", "Idle"),
            ("_x", "_x"),
            ("_1", "_1"),
            ("_", "_"),
            ("EOFX", "EOFX"),
            ("pass", "Pass"),
        ] {
            assert_eq!(capitalize_state(name.to_string()), spelled, "{name}");
        }
        assert_eq!(to_cpp_event_variant(Cow::Borrowed("EOF")), "mEOF");
    }

    #[test]
    fn a_python_member_is_never_a_name_enum_reads_as_something_else() {
        for (name, spelled) in [
            ("_x_", "m_X_"),
            ("__x__", "m__X__"),
            ("__x", "m__X"),
            ("1x_", "m_1X_"),
            ("__", "m__"),
        ] {
            assert_eq!(to_python_const(name.to_string()), spelled, "{name}");
        }
        for (name, spelled) in [
            ("_x", "_X"),
            ("1x", "_1X"),
            ("_", "_"),
            ("idle", "IDLE"),
            ("null", "NULL_"),
        ] {
            assert_eq!(to_python_const(name.to_string()), spelled, "{name}");
        }
    }
}
