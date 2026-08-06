// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Author `<param>` literals must survive every literal boundary they
// cross on the way into generated source.
//
// A `<send>` whose params are all static literals is lowered at codegen
// time: the author's text is pasted into a string literal in the
// generated file. How many literal boundaries that text crosses differs
// per backend, and the count is what makes this gate necessary rather
// than a compile check:
//
//   cpp   — one boundary. The value lands in `params["k"].push_back("v")`,
//           a C++ literal handed straight to the runtime.
//   rust  — two. The backend assembles a Lua table constructor
//   go      (`{ ["k"]="v" }`) and embeds *that* in a host string literal
//           which the script engine later parses as Lua source.
//
// A host-only escape is therefore correct for cpp and wrong for
// rust/go: it produces host source that compiles and Lua source that
// does not parse — or worse, parses to a different value. No syntax
// checker can see this. `codegen_smoke` compiles every backend and
// would stay green while the emitted Lua was unparseable, because the
// Lua never reaches a compiler. That is precisely the gap this file
// covers: it asserts the *value*, not the syntax.
//
// The Lua side is evaluated by the real Lua 5.4 interpreter (`mlua`) —
// the same one the Rust and Go runtimes use — rather than by a
// hand-rolled parser here, so the check cannot drift from the grammar
// it asserts about.
//
// Backends deliberately absent, each measured rather than assumed:
//   c11    — rejects this `<send>` shape at codegen time and emits a
//            loud `error.execution` arm, so no author text is pasted.
//   kotlin — emits no `<send>` param payload at all (under either
//            datamodel), so there is no literal to check.
//   python — passes the author *expression* through for runtime
//            evaluation instead of pasting a static value.
// Should any of them start pasting static values, add it to
// `PASTE_BACKENDS` with its extraction shape.
//
// Fixture: `tests/fixtures/codegen_smoke/send_param_adversarial_literals.scxml`,
// shared with `codegen_smoke` so one fixture feeds both the syntax gate
// and this value gate.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sce_build::compile_scxml_lang;
use sce_build::generator::Language;

/// The author `<param>` set of the shared fixture, as the values the
/// runtime must observe — post-XML-decode, post-quote-strip.
///
/// This is asserted to match the emitted key set *exactly*, so adding a
/// param to the fixture without extending this table fails rather than
/// silently going unchecked.
const EXPECTED_PARAMS: &[(&str, &str)] = &[
    // Closes the host literal mid-token when unescaped.
    ("dquote", "a\"b"),
    // `\d` is an unknown escape in C/C++/Rust/Go and in Lua: pasted raw
    // it is a warning at best and a changed value at worst.
    ("backslash", "c\\d"),
    // Backslash immediately before a quote — escaping one of the two
    // leaves the literal unbalanced.
    ("both", "e\\\"f"),
    // Terminates the Lua table constructor the Rust/Go backends build.
    ("luaclose", "g\"}h"),
    ("tab", "i\tj"),
    // A raw newline is illegal inside a Lua short literal.
    ("newline", "k\nl"),
    // The name slot is interpolated into the same literal as the value,
    // and the SCXML parser does admit a quote there.
    ("odd\"name", "plain"),
];

/// How a backend pastes static params into its output.
#[derive(Clone, Copy)]
enum PasteShape {
    /// One host literal pair per param: `params["k"].push_back("v");`.
    HostPairs { marker: &'static str },
    /// A Lua table constructor carried inside one host literal.
    LuaTableInHostLiteral { marker: &'static str },
}

/// Backends that lower a static `<param>` by pasting author text into
/// generated source. Extending this list is the whole cost of covering
/// a new backend.
const PASTE_BACKENDS: &[(Language, &str, PasteShape)] = &[
    (
        Language::Cpp,
        "cpp",
        PasteShape::HostPairs { marker: "params[" },
    ),
    (
        Language::Rust,
        "rust",
        PasteShape::LuaTableInHostLiteral {
            marker: "let event_data",
        },
    ),
    (
        Language::Go,
        "go",
        PasteShape::LuaTableInHostLiteral {
            marker: "eventDataStr",
        },
    ),
];

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codegen_smoke/send_param_adversarial_literals.scxml")
}

/// Every generated file for `lang`, concatenated. Backends split output
/// across several files (cpp emits `_sm.h` + `_sm.inl`); the paste site
/// may live in any of them.
fn generate(lang: Language) -> String {
    let fixture = fixture_path();
    let output = compile_scxml_lang(
        fixture.to_str().expect("fixture path is UTF-8"),
        &sce_build::find_template_dir_for(lang),
        lang,
    )
    .expect("codegen succeeds for the adversarial-literal fixture");

    output
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Read the `"`-delimited literal whose opening quote is at `start`.
///
/// Returns the raw body with escapes still intact, plus the index just
/// past the closing quote. A backslash consumes the character after it,
/// which is the whole point: a naive scan (or a non-greedy regex) stops
/// at the first `\"` and silently reports a truncated value, turning a
/// real defect into a passing test.
fn scan_literal(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'"') {
        return None;
    }
    let mut body = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                body.push('\\');
                i += 1;
                if i < chars.len() {
                    body.push(chars[i]);
                    i += 1;
                }
            }
            '"' => return Some((body, i + 1)),
            c => {
                body.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Undo one host-language string-literal escaping layer.
///
/// C++, Rust and Go share the escape set the generator produces
/// (`\\ \" \n \r \t`). An escape outside that set means author text was
/// pasted without escaping — `\d` from the fixture is not a valid escape
/// in any of the three — so it is reported rather than passed through.
fn unescape_host(raw: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => {
                return Err(format!(
                    "`\\{other}` is not an escape the host language defines — \
                     author text reached the literal unescaped"
                ))
            }
            None => return Err("literal ends in a dangling backslash".to_string()),
        }
    }
    Ok(out)
}

/// Extract `params["k"].push_back("v")` pairs from a host-pairs backend.
///
/// Per-param failures are accumulated rather than returned on the first
/// one: each pair is an independent paste site, and stopping at the
/// earliest would leave the rest of them unproven in exactly the run
/// that shows the defect exists.
fn extract_host_pairs(source: &str, marker: &str) -> (BTreeMap<String, String>, Vec<String>) {
    let mut found = BTreeMap::new();
    let mut errors = Vec::new();
    for line in source.lines() {
        let Some(at) = line.find(marker) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        // Byte offset → char offset: the marker is ASCII, and so is
        // everything before it on these generated lines.
        let key_quote = line[..at].chars().count() + marker.chars().count();
        let Some((raw_key, after_key)) = scan_literal(&chars, key_quote) else {
            continue;
        };
        let Some(value_quote) = chars[after_key..].iter().position(|c| *c == '"') else {
            continue;
        };
        let Some((raw_value, _)) = scan_literal(&chars, after_key + value_quote) else {
            continue;
        };
        let key = match unescape_host(&raw_key) {
            Ok(key) => key,
            Err(why) => {
                errors.push(format!("param name (raw {raw_key:?}): {why}"));
                continue;
            }
        };
        match unescape_host(&raw_value) {
            Ok(value) => {
                found.insert(key, value);
            }
            Err(why) => errors.push(format!("param value {key:?} (raw {raw_value:?}): {why}")),
        }
    }
    (found, errors)
}

/// Extract the Lua table a backend embedded in a host literal, undo the
/// host escaping, then evaluate the result with the real Lua parser.
///
/// Unlike the host-pairs shape this one is all-or-nothing by nature: the
/// params share a single table constructor, so one bad value makes the
/// whole thing unparseable and there is no per-param verdict to give.
/// The error carries the emitted Lua source so the failing run shows
/// what was actually written rather than only that something was wrong.
fn extract_lua_table(source: &str, marker: &str) -> (BTreeMap<String, String>, Vec<String>) {
    let mut tables: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for line in source.lines() {
        let Some(at) = line.find(marker) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        let after_marker = line[..at].chars().count() + marker.chars().count();
        let Some(offset) = chars[after_marker..].iter().position(|c| *c == '"') else {
            continue;
        };
        let Some((raw, _)) = scan_literal(&chars, after_marker + offset) else {
            continue;
        };
        let lua_source = match unescape_host(&raw) {
            Ok(source) => source,
            Err(why) => {
                errors.push(format!("host literal (raw {raw:?}): {why}"));
                continue;
            }
        };
        let trimmed = lua_source.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            tables.push(lua_source);
        }
    }

    if tables.len() != 1 {
        errors.push(format!(
            "expected exactly one embedded Lua table constructor, found {}",
            tables.len()
        ));
        return (BTreeMap::new(), errors);
    }

    let lua = mlua::Lua::new();
    let table: mlua::Table = match lua.load(format!("return {}", tables[0])).eval() {
        Ok(table) => table,
        Err(e) => {
            errors.push(format!(
                "the emitted Lua does not parse — host source may still compile: {e}\n  \
                 Lua source was: {}",
                tables[0]
            ));
            return (BTreeMap::new(), errors);
        }
    };

    let mut found = BTreeMap::new();
    for pair in table.pairs::<String, String>() {
        match pair {
            Ok((k, v)) => {
                found.insert(k, v);
            }
            Err(e) => errors.push(format!("Lua table entry is not string→string: {e}")),
        }
    }
    (found, errors)
}

/// Every static `<param>` value must reach the runtime byte-identical to
/// what the author wrote, on every backend that pastes it into source.
///
/// Violations are collected rather than asserted one at a time:
/// stopping at the first would hide how far a defect reaches and leave
/// the remaining backends unproven, which is exactly how the C++ break
/// and the Rust/Go break stayed separable for as long as they did.
#[test]
fn author_param_literals_survive_every_boundary() {
    let expected: BTreeMap<String, String> = EXPECTED_PARAMS
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();

    let mut violations: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (lang, name, shape) in PASTE_BACKENDS {
        let source = generate(*lang);
        let (found, errors) = match shape {
            PasteShape::HostPairs { marker } => extract_host_pairs(&source, marker),
            PasteShape::LuaTableInHostLiteral { marker } => extract_lua_table(&source, marker),
        };

        violations.extend(errors.into_iter().map(|why| format!("{name}: {why}")));

        if found.keys().ne(expected.keys()) {
            violations.push(format!(
                "{name}: param name set diverged\n  expected: {:?}\n  emitted:  {:?}",
                expected.keys().collect::<Vec<_>>(),
                found.keys().collect::<Vec<_>>(),
            ));
        }

        // Deliberately not gated on the two checks above. An unescaped
        // quote truncates one param and drops the next, so bailing out
        // on a key-set mismatch would report only the damage's edge and
        // leave the params that *were* extracted — and are wrong —
        // unexamined. Keys that failed extraction are already named
        // above; the rest still get a value verdict.
        for (key, want) in &expected {
            let Some(got) = found.get(key) else {
                continue;
            };
            checked += 1;
            if got != want {
                violations.push(format!(
                    "{name}: param {key:?} value changed crossing the literal boundary\n  \
                     authored: {want:?}\n  observed: {got:?}",
                ));
            }
        }
    }

    // Asserted before the floor: a diverged key set both suppresses the
    // count and explains why, so reporting the count first would replace
    // the diagnosis with its symptom.
    assert!(
        violations.is_empty(),
        "author `<param>` literals did not survive codegen:\n\n{}",
        violations.join("\n\n"),
    );

    // A source-derived gate that reads nothing passes vacuously. The
    // floor pins the full cross-product so a backend whose paste site
    // is renamed (and therefore no longer found) fails loudly instead of
    // quietly dropping out of coverage.
    let floor = PASTE_BACKENDS.len() * EXPECTED_PARAMS.len();
    assert_eq!(
        checked, floor,
        "expected to check {floor} backend×param pairs, checked {checked} — \
         a paste site went unread"
    );
}
