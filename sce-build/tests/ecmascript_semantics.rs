// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! What the Lua we emit answers, measured against what ECMAScript answers.
//!
//! Every case below runs the production path end to end: the ECMAScript
//! source goes through `sce_build::ecmascript`, and the Lua that comes out
//! is evaluated by `sce-rust-lua` — the engine a generated Rust state
//! machine actually runs on, with the same `_scxml_truthy` / `_typeof` /
//! `_isArray` natives and the same shared `ecma_semantics.lua`. Nothing is
//! reimplemented here, so a divergence between this file and production is
//! not expressible.
//!
//! The expected values are ECMA-262's, written out by hand in
//! `tests/ecmascript/ecma262_semantics.json`. That is the point: a golden
//! captured from our own emitter would agree with whatever we currently do,
//! including the answers that are wrong. Each case is a claim about the
//! language, and the table is where a reader can check the claim against the
//! spec rather than against us.
//!
//! The table is on disk rather than in this file because it is shared. The
//! C++ engine reads the same cases (`tests/engine/EcmaScriptSemanticsTest.cpp`)
//! — it has to, because the C++ backend emits the author's ECMAScript verbatim
//! and leaves the whole of the semantics to the engine the build selected.
//!
//! Why this exists: the transformer this replaced answered `true` for
//! `Var1 && Var2` when `Var1` was `0`, because Lua's only falsy values are
//! `nil` and `false`. Nothing failed — the guard just took the other
//! branch. The table's "truthiness" group is that defect, pinned.

use sce_build::ecmascript::{
    to_lua_condition, to_lua_script, to_lua_value, DocumentScope, ExprError,
};
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};
use serde::Deserialize;

/// What ECMA-262 says the expression evaluates to.
///
/// Externally tagged, so the shared table spells the answer's type and a
/// case cannot name two answers at once.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Answer {
    Bool(bool),
    Number(f64),
    String(String),
    /// `null` or `undefined`. ECMAScript's `==` equates the two and the SCXML
    /// datamodel has no way to tell an absent property from a null one, so
    /// they are one answer here.
    Empty(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Form {
    /// Evaluated the way a `cond=` attribute is.
    Condition,
    /// Evaluated the way an `expr=` attribute is.
    Value,
}

#[derive(Debug, Deserialize)]
struct Case {
    /// Which family of the language the case belongs to, so a failure names
    /// the area rather than only the expression.
    group: String,
    /// ECMAScript statements establishing the datamodel, run through the
    /// same `<script>` path a document would use.
    setup: String,
    /// The ECMAScript expression under test.
    source: String,
    form: Form,
    expect: Answer,
    /// The ECMA-262 clause the expectation comes from, so a reader can
    /// check the claim without trusting this file.
    clause: String,
}

#[derive(Debug, Deserialize)]
struct Table {
    cases: Vec<Case>,
}

/// The shared table, read from disk rather than compiled in.
///
/// On disk because the C++ engine reads the same file
/// (`tests/engine/EcmaScriptSemanticsTest.cpp`). Two copies of these
/// expectations would each drift toward the engine that reads them, which is
/// the exact failure this repository already had: the Rust path was measured
/// against ECMA-262 and answered correctly, the C++ path was measured against
/// nothing and answered `true` for `0 && x`, and every W3C fixture was green
/// throughout.
fn cases() -> Vec<Case> {
    let path = repo_root().join("tests/ecmascript/ecma262_semantics.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read the shared table at {}: {err}", path.display()));
    let table: Table = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{} is not a readable case table: {err}", path.display()));
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        table.cases.len() >= 55,
        "the shared ECMA-262 table produced only {} case(s), so this is not \
         measuring the corpus it claims to",
        table.cases.len()
    );
    table.cases
}

/// One row of the committed emission: the Lua a generated machine runs.
#[derive(Debug, Deserialize)]
struct Emission {
    /// The case's ECMAScript source, carried so a reader can prove the two
    /// files are still in step rather than assuming it from the ordering.
    source: String,
    setup: String,
    expression: String,
}

/// The emission the other Lua backends read, loaded here to be measured
/// against the frontend that produced it.
fn emitted_lua() -> Vec<Emission> {
    #[derive(Debug, Deserialize)]
    struct Sidecar {
        cases: Vec<Emission>,
    }
    let path = repo_root().join("tests/ecmascript/ecma262_emitted_lua.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read the emission at {}: {err}", path.display()));
    let sidecar: Sidecar = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{} is not a readable emission: {err}", path.display()));
    sidecar.cases
}

/// The declarations a case's own `setup` makes.
///
/// A case is a one-document world: `setup` is its `<script>` and `source`
/// is its `expr`, so the scope the frontend resolves against is the one
/// that chunk builds. Reusing [`DocumentScope::declare_chunk`] rather
/// than listing the names keeps this test and production on one reading
/// of what a chunk declares.
fn scope_for(setup: &str) -> DocumentScope {
    let mut scope = DocumentScope::installed();
    scope.declare_chunk(setup);
    scope
}

/// Every case, run through the emitter and then through the engine that
/// runs generated Rust.
#[test]
fn emitted_lua_answers_what_ecmascript_answers() {
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");

    let mut failures: Vec<String> = Vec::new();
    for (index, case) in cases().iter().enumerate() {
        let session = format!("case{index}");
        engine.create_session(&session);

        let scope = scope_for(&case.setup);
        if !case.setup.is_empty() {
            let script = match to_lua_script(&case.setup, &scope) {
                Ok(script) => script,
                Err(err) => {
                    failures.push(format!(
                        "[{}] setup did not parse: {err} ({})",
                        case.source, case.group
                    ));
                    continue;
                }
            };
            if let Err(err) = engine.execute_script(&session, &script) {
                failures.push(format!(
                    "[{}] setup did not run: {err}\n  emitted: {script}",
                    case.source
                ));
                continue;
            }
        }

        let emitted = match case.form {
            Form::Condition => to_lua_condition(&case.source, &scope),
            Form::Value => to_lua_value(&case.source, &scope),
        };
        let emitted = match emitted {
            Ok(emitted) => emitted,
            Err(err) => {
                failures.push(format!(
                    "[{}] did not compile: {err} ({})",
                    case.source, case.group
                ));
                continue;
            }
        };

        match engine.evaluate_expression(&session, &emitted) {
            Ok(actual) => {
                if !matches(&actual, &case.expect) {
                    failures.push(format!(
                        "[{}] answered {actual:?}, ECMAScript says {:?} ({})\n  emitted: {emitted}",
                        case.source, case.expect, case.clause
                    ));
                }
            }
            Err(err) => failures.push(format!(
                "[{}] failed to evaluate: {err}\n  emitted: {emitted}",
                case.source
            )),
        }
        engine.destroy_session(&session);
    }

    assert!(
        failures.is_empty(),
        "{} of {} expressions disagree with ECMA-262:\n{}",
        failures.len(),
        cases().len(),
        failures.join("\n")
    );
}

/// The same cases, answered by the shared library the way C11 receives it.
///
/// C11 cannot open a file at runtime, so codegen carries `ecma_semantics.lua`
/// as a sequence of `luaL_dostring` calls — and the generated code discards
/// every result, so a chunk that failed to compile leaves its definitions
/// simply absent rather than loudly missing. `shared_lua_assets.rs` asserts
/// the split loads; this asserts what it loads ANSWERS, which is the half
/// that moved when the engine vocabulary stopped being seven natives per
/// backend and became part of that file.
///
/// What this does not measure is C11's C-side value conversion. The
/// interpreter is the same Lua 5.4 the C11 build links, the chunks are the
/// ones it embeds, and the table is the shared one — the residue is the glue
/// between `lua_State` and the generated struct, which is where the C11
/// suite's own fixtures look.
#[test]
fn the_chunked_embed_answers_what_ecmascript_answers() {
    use sce_build::filters::c11_lua_chunks;

    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "c11-embed-ecma262";
    engine.create_session(session);
    for source in [
        sce_build::filters::ECMA_SEMANTICS_LUA,
        sce_build::filters::JSON_BUILTINS_LUA,
    ] {
        for chunk in c11_lua_chunks(source) {
            engine
                .execute_script(session, &chunk)
                .expect("a chunk the C11 bootstrap emits must load");
        }
    }

    let mut failures: Vec<String> = Vec::new();
    for (case, emission) in cases().iter().zip(emitted_lua()) {
        assert_eq!(
            case.source, emission.source,
            "the table and the emission are out of step"
        );
        // One session for the whole sweep, unlike the reader above: the point
        // here is the chunked load, and re-running it per case would measure
        // the same thing eighty-four times.
        if !emission.setup.is_empty() {
            if let Err(err) = engine.execute_script(session, &emission.setup) {
                failures.push(format!("[{}] setup did not run: {err}", case.source));
                continue;
            }
        }
        match engine.evaluate_expression(session, &emission.expression) {
            Ok(actual) => {
                if !matches(&actual, &case.expect) {
                    failures.push(format!(
                        "[{}] answered {actual:?}, ECMAScript says {:?} ({})",
                        case.source, case.expect, case.clause
                    ));
                }
            }
            Err(err) => failures.push(format!("[{}] failed to evaluate: {err}", case.source)),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of the shared table's cases are answered differently by the library \
         C11 embeds than by the language it claims to implement:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The Lua this frontend emits for every case, written out for the backends
/// that cannot run the frontend.
///
/// Go, Python and C11 evaluate Lua, not ECMAScript: the translation happened
/// here, at build time, and by the time those runtimes see an expression it
/// is already `_scxml_truthy(a)`. So they cannot read the shared table the
/// way the C++ and Kotlin readers do — those hand the author's source to an
/// engine that speaks the language — and until this file existed they read
/// nothing at all, which is why `ecma_semantics.lua` grew a standard library
/// that three of the six backends loaded and none of the three measured.
///
/// What the sidecar is NOT is a second set of expectations. The answers stay
/// in `ecma262_semantics.json`, one copy, checked against ECMA-262 clauses;
/// this file carries only the emission, so a reader on another backend is
/// measuring its own runtime library and its own Lua interpreter against the
/// same claims. The emission is derived, so it is regenerated rather than
/// edited:
///
/// ```text
/// UPDATE_EXPECT=1 cargo test -p sce-build --test ecmascript_semantics
/// ```
///
/// and the assertion below is what keeps a stale copy from reporting green on
/// the other three backends after the frontend changes.
#[test]
fn the_emitted_lua_every_backend_reads_is_what_this_frontend_emits() {
    let cases = cases();
    let mut rows = Vec::new();
    for case in &cases {
        let scope = scope_for(&case.setup);
        let setup = if case.setup.is_empty() {
            String::new()
        } else {
            to_lua_script(&case.setup, &scope)
                .unwrap_or_else(|err| panic!("[{}] setup did not compile: {err}", case.source))
        };
        let expression = match case.form {
            Form::Condition => to_lua_condition(&case.source, &scope),
            Form::Value => to_lua_value(&case.source, &scope),
        }
        .unwrap_or_else(|err| panic!("[{}] did not compile: {err}", case.source));
        rows.push(serde_json::json!({
            "source": case.source,
            "setup": setup,
            "expression": expression,
        }));
    }

    let document = serde_json::json!({
        "about": [
            "The Lua `sce-build`'s ECMAScript frontend emits for every case in",
            "ecma262_semantics.json, in the same order, cross-checked by `source`.",
            "",
            "GENERATED. Regenerate with:",
            "  UPDATE_EXPECT=1 cargo test -p sce-build --test ecmascript_semantics",
            "",
            "It exists because a backend that runs Lua never sees the ECMAScript:",
            "the translation happens at build time, so Go, Python and C11 cannot",
            "evaluate `a === b` the way the C++ and Kotlin engines do. They read",
            "this beside the table and measure their own runtime library and Lua",
            "interpreter against the table's ECMA-262 answers.",
            "",
            "The answers are NOT here. One copy of those, in ecma262_semantics.json,",
            "with the clause each comes from — a per-backend copy would drift",
            "toward the backend that reads it.",
        ],
        "cases": rows,
    });
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&document).expect("the sidecar serialises")
    );

    let path = repo_root().join("tests/ecmascript/ecma262_emitted_lua.json");
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(&path, &rendered)
            .unwrap_or_else(|err| panic!("cannot write {}: {err}", path.display()));
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read {}: {err}\nRegenerate it with \
             `UPDATE_EXPECT=1 cargo test -p sce-build --test ecmascript_semantics`",
            path.display()
        )
    });
    assert_eq!(
        committed, rendered,
        "the committed emission at tests/ecmascript/ecma262_emitted_lua.json is not \
         what this frontend emits any more. Go, Python and C11 read that file to \
         measure their runtimes, so a stale copy does not fail here — it reports \
         green on three backends while they evaluate Lua the frontend stopped \
         emitting. Regenerate with \
         `UPDATE_EXPECT=1 cargo test -p sce-build --test ecmascript_semantics`."
    );
}

/// Every ECMAScript expression the repository commits, compiled — and the
/// handful the frontend refuses named one by one.
///
/// A refusal is not a build failure (W3C §5.9.1 makes it a runtime
/// `error.execution`; see `filters::to_lua_guard`), which is exactly why it
/// has to be counted here: a rejection that grew a new member would
/// otherwise show up as a document that silently started raising at runtime
/// instead of evaluating. The expected set is written out, so adding to it
/// is a decision somebody makes rather than a number that drifts.
///
/// The scan set is `git ls-files`, not a directory list: a sweep that names
/// trees reports success over whatever subset still exists.
#[test]
fn every_committed_ecmascript_expression_compiles() {
    let mut compiled = 0usize;
    let mut documents = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in committed_documents() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = strip_xml_comments(&text);
        // Forge documents are the *other* dialect — typed Extended SCXML,
        // handled by `crate::forge::expr` — and their expressions are not
        // ECMAScript.
        if text.contains("sce:kind") {
            continue;
        }
        documents += 1;
        let label = path
            .strip_prefix(repo_root())
            .unwrap_or(&path)
            .display()
            .to_string();

        for (attribute, source) in expression_attributes(&text) {
            // A preprocessor template carries `{$name}` placeholders that
            // are substituted before the document reaches codegen, so its
            // pre-substitution text is not ECMAScript and never was.
            if source.contains("{$") {
                continue;
            }
            // A native guard (`cpp:`/`kt:`) is evaluated by the host
            // language, not the script engine — the same boundary
            // `parser::check_expression_needs` draws, mirrored here so this
            // sweep measures exactly the set that reaches the Lua path.
            if source.starts_with("cpp:") || source.starts_with("kt:") {
                continue;
            }
            let result = if attribute == "cond" {
                compiles_as_condition(&source)
            } else {
                compiles_as_value(&source)
            };
            match result {
                Ok(_) => compiled += 1,
                Err(err) => failures.push(format!("{label}: {attribute}=\"{source}\" -> {err}")),
            }
        }

        for body in script_bodies(&text) {
            // `<script><cpp>…</cpp></script>` is a native action, not
            // ECMAScript; the parser never sees it and neither does this.
            if body.contains("<cpp>") || body.trim().is_empty() {
                continue;
            }
            match compiles_as_script(&body) {
                Ok(_) => compiled += 1,
                Err(err) => failures.push(format!("{label}: <script> -> {err}")),
            }
        }
    }

    eprintln!("ecmascript sweep: {documents} document(s), {compiled} expression(s)");

    // Measured 2026-08-14: 913 committed `.scxml`/`.txml`, of which 723 are
    // ECMAScript documents (the rest declare a Forge kind) carrying 821
    // expressions and script bodies — all of which compile. The floors sit
    // under those numbers so a retired fixture does not raise a false alarm
    // while a sweep that silently stopped finding anything still cannot pass.
    assert!(
        documents >= 650,
        "the sweep read only {documents} documents, so it is not measuring \
         the corpus it claims to"
    );
    assert!(
        compiled >= 750,
        "the sweep compiled only {compiled} expressions across {documents} \
         documents — check the attribute scan before reading its verdict"
    );
    // The corpus's unparseable expressions, and they are unparseable on
    // purpose: W3C tests 309 and 344 write `cond="return"` to check that a
    // processor raises error.execution and reads the condition as false.
    // ECMA-262 12.7.2 makes `return` a reserved word, so refusing it is the
    // correct verdict — and the seam turns the refusal into the runtime
    // error the tests are looking for.
    let expected: Vec<&str> = vec![
        "resources/309/test309.scxml: cond=\"return\"",
        "resources/344/test344.scxml: cond=\"return\"",
    ];
    let unexpected: Vec<&String> = failures
        .iter()
        .filter(|f| !expected.iter().any(|e| f.starts_with(e)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{} committed expression(s) the ECMAScript frontend refuses that were \
         not expected to be refused. A refusal is emitted as Lua that raises \
         at evaluation (W3C §5.9.1), so this does not fail the build — which \
         is why it has to fail here:\n{}",
        unexpected.len(),
        unexpected
            .iter()
            .map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    for entry in &expected {
        assert!(
            failures.iter().any(|f| f.starts_with(entry)),
            "`{entry}` is listed as an expected refusal but the frontend now \
             accepts it — if that is intended, drop the entry; if it is not, \
             the reserved-word check has stopped firing"
        );
    }
}

/// Whatever the Forge dialect accepts, this dialect accepts too.
///
/// The two parsers share a lexer and nothing above it, which is what keeps
/// the typed backends from having to contemplate array literals and `new`.
/// The containment is the price of that split: Extended SCXML is a subset
/// of ECMAScript, so a Forge expression that this frontend rejects means
/// the two have drifted apart on the grammar they are supposed to share.
#[test]
fn every_forge_expression_also_parses_as_ecmascript() {
    // Shapes drawn from the Forge fixtures: typed comparisons, member
    // paths, calls, arithmetic with coercion, ternaries, bit operations.
    const FORGE_SHAPES: &[&str] = &[
        "a === b",
        "a !== 'ack'",
        "_event.data.value > 3",
        "frame.encode(payload)",
        "(a + b) * 2 - 1",
        "a ? b : c",
        "flags & 0x0F",
        "(crc << 1) ^ 0x1021",
        "len(items)",
        "items[0].pattern",
        "a && b || !c",
        "temp / 5 + 32.0",
        "value >>> 2",
        "~mask",
    ];
    let mut refused = Vec::new();
    for shape in FORGE_SHAPES {
        if let Err(err) = compiles_as_value(shape) {
            refused.push(format!("{shape} -> {err}"));
        }
    }
    assert!(
        refused.is_empty(),
        "the ECMAScript dialect refused expressions the Forge dialect accepts, \
         so the two have drifted:\n{}",
        refused.join("\n")
    );
}

// ── Grammar without names ──────────────────────────────────────
//
// Two sweeps below ask whether an expression *compiles* — whether the
// grammar admits it and the emitter has a shape for it. They read source
// out of documents (and out of a literal table) without the document, so
// they cannot ask the other question the frontend answers: whether the
// names it mentions are declared. That one is asked over the same corpus
// by `ecmascript_acceptance_parity`, which builds the model and therefore
// has a scope to ask it with.
//
// Calling the two halves directly rather than through
// `to_lua_value` is what keeps the distinction honest: a sweep handed
// `DocumentScope::installed()` would report every `Var1` in the corpus as
// undeclared and would be measuring the absence of its own input.

fn compiles_as_value(source: &str) -> Result<String, ExprError> {
    let ast = sce_build::ecmascript::parser::parse_expression(source)?;
    sce_build::ecmascript::lua::emit_value(&ast)
}

fn compiles_as_condition(source: &str) -> Result<String, ExprError> {
    let ast = sce_build::ecmascript::parser::parse_expression(source)?;
    sce_build::ecmascript::lua::emit_condition(&ast)
}

fn compiles_as_script(source: &str) -> Result<String, ExprError> {
    let stmts = sce_build::ecmascript::parser::parse_script(source)?;
    sce_build::ecmascript::lua::emit_script(&stmts)
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn committed_documents() -> Vec<std::path::PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["ls-files", "*.scxml", "*.txml"])
        .output()
        .expect("git ls-files");
    assert!(
        out.status.success(),
        "git ls-files failed, so this sweep has no scan set to measure"
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|rel| repo_root().join(rel))
        .collect()
}

fn strip_xml_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Every `cond=` / `expr=` attribute value, XML-unescaped.
fn expression_attributes(text: &str) -> Vec<(&'static str, String)> {
    let mut found = Vec::new();
    for name in ["cond", "expr"] {
        let needle = format!("{name}=");
        let mut rest = text;
        while let Some(at) = rest.find(&needle) {
            // `srcexpr=` and `nameexpr=` end in `expr=` without being it.
            let preceded_by_word = rest[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':');
            let after = &rest[at + needle.len()..];
            let quote = after.chars().next();
            rest = after;
            let Some(quote) = quote else { break };
            if quote != '"' && quote != '\'' {
                continue;
            }
            let body = &after[1..];
            let Some(close) = body.find(quote) else { break };
            rest = &body[close + 1..];
            if preceded_by_word {
                continue;
            }
            let value = unescape_xml(&body[..close]);
            if !value.trim().is_empty() {
                found.push((name, value));
            }
        }
    }
    found
}

fn script_bodies(text: &str) -> Vec<String> {
    let mut bodies = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("<script") {
        let after = &rest[open..];
        let Some(gt) = after.find('>') else { break };
        // `<script src="…"/>` has no body.
        if after[..gt].ends_with('/') {
            rest = &after[gt + 1..];
            continue;
        }
        let body_start = &after[gt + 1..];
        let Some(close) = body_start.find("</script>") else {
            break;
        };
        bodies.push(unescape_xml(&body_start[..close]));
        rest = &body_start[close + 9..];
    }
    bodies
}

fn unescape_xml(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// An engine may hold a whole number as an integer or as a double, and
/// ECMA-262 has one Number type — so both spellings answer a `number` case.
fn matches(actual: &ScriptValue, expected: &Answer) -> bool {
    match (actual, expected) {
        (ScriptValue::Bool(a), Answer::Bool(b)) => a == b,
        (ScriptValue::Int(a), Answer::Number(b)) => (*a as f64 - b).abs() < f64::EPSILON,
        (ScriptValue::Double(a), Answer::Number(b)) => (a - b).abs() < 1e-9,
        (ScriptValue::String(a), Answer::String(b)) => a == b,
        (ScriptValue::Null | ScriptValue::Undefined, Answer::Empty(_)) => true,
        _ => false,
    }
}
