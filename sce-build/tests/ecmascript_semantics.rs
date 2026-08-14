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
//! The expected values are ECMA-262's, written out by hand. That is the
//! point: a golden captured from our own emitter would agree with whatever
//! we currently do, including the answers that are wrong. Each case is a
//! claim about the language, and the table is where a reader can check the
//! claim against the spec rather than against us.
//!
//! Why this exists: the transformer this replaced answered `true` for
//! `Var1 && Var2` when `Var1` was `0`, because Lua's only falsy values are
//! `nil` and `false`. Nothing failed — the guard just took the other
//! branch. Cases in the "falsy" group below are that defect, pinned.

use sce_build::ecmascript::{to_lua_condition, to_lua_script, to_lua_value};
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};

/// What ECMA-262 says the expression evaluates to.
#[derive(Debug, Clone, PartialEq)]
enum Answer {
    Bool(bool),
    Num(f64),
    Text(&'static str),
    Empty,
}

struct Case {
    /// ECMAScript statements establishing the datamodel, run through the
    /// same `<script>` path a document would use.
    setup: &'static str,
    /// The ECMAScript expression under test.
    source: &'static str,
    /// Emitted as a condition (`cond=`) rather than for its value.
    as_condition: bool,
    expected: Answer,
    /// The ECMA-262 clause the expectation comes from, so a reader can
    /// check the claim without trusting this file.
    clause: &'static str,
}

const fn value_case(
    setup: &'static str,
    source: &'static str,
    expected: Answer,
    clause: &'static str,
) -> Case {
    Case {
        setup,
        source,
        as_condition: false,
        expected,
        clause,
    }
}

const fn cond_case(
    setup: &'static str,
    source: &'static str,
    expected: bool,
    clause: &'static str,
) -> Case {
    Case {
        setup,
        source,
        as_condition: true,
        expected: Answer::Bool(expected),
        clause,
    }
}

fn cases() -> Vec<Case> {
    vec![
        // ── Truthiness: ECMAScript's falsy set is wider than Lua's ──────
        // Lua counts only `nil` and `false` as false, so every case here is
        // one the string-rewriting transformer got wrong.
        cond_case("var a = 0;", "a", false, "7.1.2 ToBoolean(0) = false"),
        cond_case("var a = '';", "a", false, "7.1.2 ToBoolean('') = false"),
        cond_case("var a = 0;", "!a", true, "12.5.9 logical NOT"),
        cond_case(
            "var a = 0; var b = 5;",
            "a && b",
            false,
            "13.13.1 the AND yields its left operand when it is falsy",
        ),
        cond_case(
            "var a = 0; var b = 5;",
            "a || b",
            true,
            "13.13.1 the OR yields its right operand when the left is falsy",
        ),
        value_case(
            "var a = 0; var b = 5;",
            "a && b",
            Answer::Num(0.0),
            "13.13.1 the value is an operand, not a boolean",
        ),
        value_case(
            "var a = 0; var b = 5;",
            "a || b",
            Answer::Num(5.0),
            "13.13.1 the value is an operand, not a boolean",
        ),
        value_case(
            "var a = '';",
            "a ? 'yes' : 'no'",
            Answer::Text("no"),
            "13.14 conditional uses ToBoolean",
        ),
        value_case(
            "var a = false;",
            "true ? a : 'other'",
            Answer::Bool(false),
            "13.14 a false consequent is still the result",
        ),
        // ── `+` is concatenation or addition depending on the operands ──
        value_case("", "1 + 2", Answer::Num(3.0), "13.15.3 numeric addition"),
        value_case(
            "",
            "'a' + 'b'",
            Answer::Text("ab"),
            "13.15.3 string concatenation",
        ),
        value_case(
            "",
            "1 + ''",
            Answer::Text("1"),
            "7.1.17 ToString(1) is '1', not '1.0'",
        ),
        value_case(
            "",
            "'n' + 2",
            Answer::Text("n2"),
            "13.15.3 either side a string makes it concatenation",
        ),
        value_case(
            "var a = [1,2];",
            "a + ''",
            Answer::Text("1,2"),
            "7.1.1 ToPrimitive of an Array joins with commas",
        ),
        // ── `==` coerces where `===` does not ───────────────────────────
        cond_case("", "1 == '1'", true, "7.2.15 number vs string coerces"),
        cond_case("", "1 === '1'", false, "7.2.16 strict equality does not"),
        cond_case("", "0 == false", true, "7.2.15 boolean is ToNumber'd"),
        cond_case("", "'' == 0", true, "7.2.15 empty string is ToNumber 0"),
        cond_case("", "1 != '1'", false, "7.2.15 negation of the above"),
        cond_case(
            "var a = [1,2,3];",
            "a == 0",
            false,
            "7.2.15 array vs number: ToPrimitive gives '1,2,3', ToNumber gives NaN",
        ),
        cond_case(
            "var a = null;",
            "a == null",
            true,
            "7.2.15 null and undefined equal each other",
        ),
        // ── `%` truncates toward zero; Lua's `%` floors ─────────────────
        value_case("", "-7 % 3", Answer::Num(-1.0), "13.7 remainder truncates"),
        value_case("", "7 % 3", Answer::Num(1.0), "13.7 remainder"),
        // ── typeof / instanceof ─────────────────────────────────────────
        cond_case(
            "var a = 1;",
            "typeof a !== 'undefined'",
            true,
            "13.5.3 typeof of a bound variable",
        ),
        cond_case(
            "",
            "typeof missingVariable !== 'undefined'",
            false,
            "13.5.3 typeof of an unresolvable reference is 'undefined'",
        ),
        value_case(
            "var a = 'x';",
            "typeof a",
            Answer::Text("string"),
            "13.5.3 typeof table",
        ),
        cond_case(
            "var a = [1,2,3];",
            "a instanceof Array",
            true,
            "13.10.2 instanceof Array",
        ),
        cond_case(
            "var a = 3;",
            "a instanceof Array",
            false,
            "13.10.2 a number is not an Array",
        ),
        // ── Arrays and objects ──────────────────────────────────────────
        value_case(
            "var a = [1,2,3];",
            "a.length",
            Answer::Num(3.0),
            "23.1.3 Array length",
        ),
        value_case("", "'abc'.length", Answer::Num(3.0), "22.1.3 String length"),
        value_case(
            "var a = [1,2,3];",
            "[].concat(a, [4]).length",
            Answer::Num(4.0),
            "23.1.3.1 concat appends every argument, not just the first",
        ),
        value_case(
            "var a = [1,2,3];",
            "a.indexOf(2)",
            Answer::Num(1.0),
            "23.1.3.14 indexOf is zero-based",
        ),
        value_case(
            "var o = {k: 'v'};",
            "o.k",
            Answer::Text("v"),
            "13.3.2 member access on an object literal",
        ),
        value_case(
            "var o = {'a b': 7};",
            "o['a b']",
            Answer::Num(7.0),
            "13.3.3 computed member access with a non-identifier key",
        ),
        value_case(
            "var o = {k: 1};",
            "o.missing",
            Answer::Empty,
            "13.3.2 a property that is not there reads as undefined",
        ),
        // An Array is stored as a 1-based Lua sequence, so every numeric
        // index moves by one — and a string key must not.
        value_case(
            "var a = [10,20,30];",
            "a[0]",
            Answer::Num(10.0),
            "23.1 array indices are zero-based",
        ),
        value_case(
            "var a = [10,20,30]; var i = 2;",
            "a[i]",
            Answer::Num(30.0),
            "13.3.3 a computed index is still zero-based",
        ),
        value_case(
            "var o = {k: 7}; var s = 'k';",
            "o[s]",
            Answer::Num(7.0),
            "13.3.3 a computed string key addresses a property, unshifted",
        ),
        // ── Side-effecting expressions ──────────────────────────────────
        cond_case(
            "var v = 1;",
            "++v == 2",
            true,
            "13.4.4 prefix increment yields the new value",
        ),
        value_case(
            "var v = 1;",
            "v++",
            Answer::Num(1.0),
            "13.4.2 postfix increment yields the old value",
        ),
        value_case(
            "var v = '1';",
            "++v",
            Answer::Num(2.0),
            "13.4.4 the operand is ToNumber'd, so this is not concatenation",
        ),
        // ── Functions, `new`, and scripts ───────────────────────────────
        value_case(
            "var f = function(x) { return x + 1; };",
            "f(2)",
            Answer::Num(3.0),
            "15.2 a function expression is a value",
        ),
        value_case(
            "function ctor() { this.bar = 0; } var o = new ctor();",
            "o.bar",
            Answer::Num(0.0),
            "13.3.5 new binds `this` to a fresh object and yields it",
        ),
        value_case(
            "var Var1 = 1; Var1 += 1;",
            "Var1",
            Answer::Num(2.0),
            "13.15.2 compound assignment",
        ),
        value_case(
            "var total = 0; for (var i = 0; i < 4; i++) { total += i; }",
            "total",
            Answer::Num(6.0),
            "14.7.4 the for statement",
        ),
        value_case(
            "var n = 0; var i = 0; while (i < 3) { i++; if (i === 2) { continue; } n += i; }",
            "n",
            Answer::Num(4.0),
            "14.7.3 while, with continue skipping one iteration",
        ),
        // ── Bitwise operates on ToInt32, not on Lua integers ────────────
        value_case(
            "",
            "5 & 3",
            Answer::Num(1.0),
            "13.12 bitwise AND over ToInt32",
        ),
        value_case("", "5 | 3", Answer::Num(7.0), "13.12 bitwise OR"),
        value_case("", "5 ^ 3", Answer::Num(6.0), "13.12 bitwise XOR"),
        value_case("", "~5", Answer::Num(-6.0), "13.5.6 bitwise NOT"),
        value_case("", "1 << 3", Answer::Num(8.0), "13.9.1 left shift"),
        value_case(
            "",
            "-8 >> 1",
            Answer::Num(-4.0),
            "13.9.2 signed right shift",
        ),
        value_case(
            "",
            "-8 >>> 28",
            Answer::Num(15.0),
            "13.9.3 unsigned right shift",
        ),
        value_case(
            "var a = 1; var b = 2;",
            "(a == 1) | (b != 2)",
            Answer::Num(1.0),
            "13.12 booleans are ToInt32'd, so the result is a number",
        ),
        // ── Values that stay themselves ─────────────────────────────────
        value_case("", "'it\\'s'", Answer::Text("it's"), "12.9.4 escaped quote"),
        value_case(
            "",
            "'\\u00e9'",
            Answer::Text("é"),
            "12.9.4 unicode escape decodes to the character",
        ),
        // Reading an *undeclared* identifier is a ReferenceError in
        // ECMAScript, and SCE's engine raises one too, so the null case is
        // asked of a declared variable — which is what `<data id="a"/>`
        // produces.
        cond_case("var a = null;", "!a", true, "7.1.2 ToBoolean(null) = false"),
        value_case(
            "",
            "+'3'",
            Answer::Num(3.0),
            "13.5.4 unary plus is ToNumber",
        ),
    ]
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

        if !case.setup.is_empty() {
            let script = match to_lua_script(case.setup) {
                Ok(script) => script,
                Err(err) => {
                    failures.push(format!("[{}] setup did not parse: {err}", case.source));
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

        let emitted = if case.as_condition {
            to_lua_condition(case.source)
        } else {
            to_lua_value(case.source)
        };
        let emitted = match emitted {
            Ok(emitted) => emitted,
            Err(err) => {
                failures.push(format!("[{}] did not compile: {err}", case.source));
                continue;
            }
        };

        match engine.evaluate_expression(&session, &emitted) {
            Ok(actual) => {
                if !matches(&actual, &case.expected) {
                    failures.push(format!(
                        "[{}] answered {actual:?}, ECMAScript says {:?} ({})\n  emitted: {emitted}",
                        case.source, case.expected, case.clause
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
                to_lua_condition(&source)
            } else {
                to_lua_value(&source)
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
            match to_lua_script(&body) {
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
        if let Err(err) = to_lua_value(shape) {
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

fn matches(actual: &ScriptValue, expected: &Answer) -> bool {
    match (actual, expected) {
        (ScriptValue::Bool(a), Answer::Bool(b)) => a == b,
        (ScriptValue::Int(a), Answer::Num(b)) => (*a as f64 - b).abs() < f64::EPSILON,
        (ScriptValue::Double(a), Answer::Num(b)) => (a - b).abs() < 1e-9,
        (ScriptValue::String(a), Answer::Text(b)) => a == b,
        (ScriptValue::Null | ScriptValue::Undefined, Answer::Empty) => true,
        _ => false,
    }
}
