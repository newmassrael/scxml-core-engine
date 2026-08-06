// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Author `<param>` literals must survive every literal boundary they
// cross on the way into generated source.
//
// A `<send>` whose params are all static literals is lowered at codegen
// time: the author's text is pasted into a string literal in the
// generated file. Two things vary per paste site, and both are axes of
// this gate rather than assumptions:
//
//   how many boundaries — cpp pastes into one host literal; the Rust and
//     Go event-data path assembles a Lua table constructor and embeds
//     *that* in a host literal, so the text crosses two. A host-only
//     escape is correct for the first and wrong for the second: it
//     yields source that compiles and Lua that does not parse.
//
//   which site is reached — the backends route `#_internal` / `#_parent`
//     / delayed-parent / BasicHTTP / delayed / immediate sends through
//     separate emission blocks, and select different blocks again on
//     `needs_script_engine`. Those blocks were measured to escape
//     inconsistently. Before the fixtures below crossed both axes this
//     gate held exactly one of C++'s six paste sites, while the other
//     five were fixed but unguarded — a fix without a witness.
//
// No syntax checker can cover the first axis. `codegen_smoke` compiles
// every backend and was measured to stay 7/7 green while the emitted Lua
// was unparseable, because that Lua never reaches a compiler. This file
// asserts the *value*, not the syntax; the two gates are complementary
// and share fixtures so one document feeds both.
//
// The Lua side is evaluated by the real Lua 5.4 interpreter (`mlua`) —
// the same one the Rust and Go runtimes use — rather than by a
// hand-rolled parser here, so the check cannot drift from the grammar
// it asserts about.
//
// What this gate deliberately does not model: which backend emits which
// shape. That cross-product is real (Kotlin drops non-HTTP params unless
// a script engine is needed; C++ routes some shapes to runtime
// evaluation once one is) but encoding it here would be a second,
// unverified copy of the templates' branching. Instead every paste site
// found is checked, and `MIN_CHECKS_PER_BACKEND` keeps a site that stops
// being emitted from silently reducing coverage.
//
// Backends deliberately absent, each measured rather than assumed:
//   c11    — pastes param names into Lua *identifier* position
//            (`_pending_donedata.<name>`), which no escaping fixes; it
//            needs index syntax. Tracked separately; its compile break
//            is caught by `codegen_smoke`.
//   python — already correct, and the reference the others were brought
//            up to: `py_string_literal` is applied at every param slot,
//            name and value alike, and the emitted module compiles.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sce_build::compile_scxml_lang;
use sce_build::generator::Language;

/// Fixture stems under `tests/fixtures/codegen_smoke/`, shared with
/// `codegen_smoke`. The pair exists because `needs_script_engine`
/// selects mutually exclusive emission blocks: one document cannot
/// reach both, so one document cannot prove both.
const FIXTURES: &[&str] = &[
    "send_param_adversarial_literals",
    "send_param_adversarial_literals_scripted",
];

/// `<send>` shape prefixes used by the fixtures' param names.
const SHAPES: &[&str] = &[
    "internal", "parent", "pdelay", "http", "httpmix", "texpr", "mix", "delay", "plain",
];

/// The adversarial value set, applied to every shape in both fixtures.
/// Generated keys are `<shape>_<suffix>`.
///
/// The suffix doubles as the param *name*, so `odd"name` also proves the
/// name slot is escaped — the SCXML parser admits a quote there, which
/// was measured rather than assumed.
const VALUES: &[(&str, &str)] = &[
    // Closes the host literal mid-token when unescaped.
    ("dquote", "a\"b"),
    // `\d` is an unknown escape in C/C++/Rust/Go/Kotlin and in Lua:
    // pasted raw it is a warning at best and a changed value at worst.
    ("backslash", "c\\d"),
    // Backslash immediately before a quote — escaping one of the two
    // leaves the literal unbalanced.
    ("both", "e\\\"f"),
    // Terminates the Lua table constructor the Rust/Go backends build.
    ("luaclose", "g\"}h"),
    ("tab", "i\tj"),
    // A raw newline is illegal inside a Lua short literal.
    ("newline", "k\nl"),
    // A quote inside the name slot itself.
    ("odd\"name", "plain"),
];

/// How a paste site presents itself in generated source.
#[derive(Clone, Copy)]
enum Extractor {
    /// A line carrying key and value as separate host literals.
    /// `key_at` / `value_at` index the literals following `marker`,
    /// which differs per backend: Go's HTTP form repeats the key
    /// (`m["k"] = append(m["k"], "v")`), so its value sits at index 2.
    HostPairs {
        marker: &'static str,
        key_at: usize,
        value_at: usize,
    },
    /// A Lua table constructor carried inside one host string literal.
    LuaTable { marker: &'static str },
}

struct Backend {
    lang: Language,
    name: &'static str,
    extractors: &'static [Extractor],
}

const PASTE_BACKENDS: &[Backend] = &[
    Backend {
        lang: Language::Cpp,
        name: "cpp",
        extractors: &[
            Extractor::HostPairs {
                marker: "params[",
                key_at: 0,
                value_at: 1,
            },
            Extractor::HostPairs {
                marker: "httpParams[",
                key_at: 0,
                value_at: 1,
            },
        ],
    },
    Backend {
        lang: Language::Rust,
        name: "rust",
        extractors: &[
            Extractor::LuaTable {
                marker: "let event_data",
            },
            Extractor::HostPairs {
                marker: "http_params.entry(",
                key_at: 0,
                value_at: 1,
            },
            // Runtime-assembled event data: the format string occupies
            // literal 0, so key and value shift to 1 and 2.
            Extractor::HostPairs {
                marker: "parts.push(format!(",
                key_at: 1,
                value_at: 2,
            },
        ],
    },
    Backend {
        lang: Language::Go,
        name: "go",
        extractors: &[
            Extractor::LuaTable {
                marker: "eventDataStr",
            },
            Extractor::HostPairs {
                marker: "httpParams[",
                key_at: 0,
                value_at: 2,
            },
            // Runtime-assembled event data: the format string occupies
            // literal 0, so key and value shift to 1 and 2.
            Extractor::HostPairs {
                marker: "fmt.Sprintf(",
                key_at: 1,
                value_at: 2,
            },
        ],
    },
    Backend {
        lang: Language::Kotlin,
        name: "kotlin",
        extractors: &[
            Extractor::HostPairs {
                marker: "httpParams[",
                key_at: 0,
                value_at: 1,
            },
            Extractor::HostPairs {
                marker: "paramsP[",
                key_at: 0,
                value_at: 1,
            },
            Extractor::HostPairs {
                marker: "paramsT[",
                key_at: 0,
                value_at: 1,
            },
            Extractor::HostPairs {
                marker: "paramsE[",
                key_at: 0,
                value_at: 1,
            },
        ],
    },
];

/// Per-backend floor on how many `<param>` values this gate must have
/// verified across both fixtures. Measured, not guessed: it is the count
/// the current templates produce. A paste site that stops being emitted
/// — renamed marker, branch removed, shape dropped — pushes the count
/// below its floor and fails, which is the only thing standing between
/// "the gate is green" and "the gate read nothing".
const MIN_CHECKS_PER_BACKEND: &[(&str, usize)] =
    &[("cpp", 56), ("rust", 119), ("go", 119), ("kotlin", 84)];

fn fixture_path(stem: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codegen_smoke")
        .join(format!("{stem}.scxml"))
}

/// Every generated file for `lang`, concatenated. Backends split output
/// across several files (cpp emits `_sm.h` + `_sm.inl`); the paste site
/// may live in any of them.
fn generate(lang: Language, stem: &str) -> String {
    let fixture = fixture_path(stem);
    let output = compile_scxml_lang(
        fixture.to_str().expect("fixture path is UTF-8"),
        &sce_build::find_template_dir_for(lang),
        lang,
    )
    .unwrap_or_else(|e| panic!("codegen succeeds for {stem}: {e}"));

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

/// Every `"`-delimited literal on a line at or after char offset `from`,
/// in source order, escapes intact.
fn literals_after(chars: &[char], from: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '"' {
            match scan_literal(chars, i) {
                Some((body, next)) => {
                    out.push(body);
                    i = next;
                }
                None => break,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Undo one host-language string-literal escaping layer.
///
/// C++, Rust, Go and Kotlin share the escape set the generator produces
/// (`\\ \" \n \r \t`). An escape outside that set means author text was
/// pasted without escaping — `\d` from the fixture is not a valid escape
/// in any of them — so it is reported rather than passed through.
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

fn char_offset_past(line: &str, byte_at: usize, marker: &str) -> usize {
    line[..byte_at].chars().count() + marker.chars().count()
}

/// Is this a key the fixtures authored — `<shape>_<suffix>`?
///
/// Generated code carries plenty of other string pairs; only the ones
/// traceable to a fixture `<param>` are this gate's business.
fn authored_value_for(key: &str) -> Option<&'static str> {
    let rest = SHAPES
        .iter()
        .find_map(|shape| key.strip_prefix(shape)?.strip_prefix('_'))?;
    VALUES
        .iter()
        .find(|(suffix, _)| *suffix == rest)
        .map(|(_, value)| *value)
}

/// Collected `(key, observed value, where)` triples plus extraction
/// failures, from every line carrying `marker`.
///
/// A line whose literal at `value_at` is absent, or is empty, is
/// skipped rather than reported: the same marker also matches
/// runtime-evaluation lines — `push_back(resultToString(...))` and its
/// `push_back("")` error fallback — which paste no author text at all.
/// No fixture value is empty, so an empty literal is unambiguously that
/// fallback and never a lost value. Missing coverage is the floor's job;
/// this function only judges what it can actually read.
fn extract_host_pairs(
    source: &str,
    marker: &str,
    key_at: usize,
    value_at: usize,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for line in source.lines() {
        let Some(at) = line.find(marker) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        let literals = literals_after(&chars, char_offset_past(line, at, marker));
        let (Some(raw_key), Some(raw_value)) = (literals.get(key_at), literals.get(value_at))
        else {
            continue;
        };
        let key = match unescape_host(raw_key) {
            Ok(key) => key,
            Err(why) => {
                errors.push(format!("param name (raw {raw_key:?}): {why}"));
                continue;
            }
        };
        if authored_value_for(&key).is_none() {
            continue;
        }
        match unescape_host(raw_value) {
            Ok(value) if value.is_empty() => continue,
            Ok(value) => found.push((key, value)),
            Err(why) => errors.push(format!("param value {key:?} (raw {raw_value:?}): {why}")),
        }
    }
    (found, errors)
}

/// Extract every Lua table a backend embedded in a host literal, undo the
/// host escaping, then evaluate each with the real Lua parser.
///
/// Per-table this is all-or-nothing by nature: the params of one `<send>`
/// share a single constructor, so one bad value makes the whole thing
/// unparseable and there is no per-param verdict to give. The error
/// carries the emitted Lua so a failing run shows what was written.
fn extract_lua_tables(source: &str, marker: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut errors: Vec<String> = Vec::new();
    let mut found = Vec::new();
    let lua = mlua::Lua::new();

    for line in source.lines() {
        let Some(at) = line.find(marker) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        let Some(raw) = literals_after(&chars, char_offset_past(line, at, marker))
            .into_iter()
            .next()
        else {
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
        if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
            continue;
        }

        let table: mlua::Table = match lua.load(format!("return {lua_source}")).eval() {
            Ok(table) => table,
            Err(e) => {
                errors.push(format!(
                    "the emitted Lua does not parse — host source may still compile: {e}\n  \
                     Lua source was: {lua_source}"
                ));
                continue;
            }
        };
        for pair in table.pairs::<String, String>() {
            match pair {
                Ok((k, v)) => {
                    if authored_value_for(&k).is_some() {
                        found.push((k, v));
                    }
                }
                Err(e) => errors.push(format!("Lua table entry is not string→string: {e}")),
            }
        }
    }
    (found, errors)
}

/// Every static `<param>` value must reach the runtime byte-identical to
/// what the author wrote, on every backend that pastes it into source,
/// for every `<send>` shape and datamodel branch that backend lowers.
#[test]
fn author_param_literals_survive_every_boundary() {
    let floors: BTreeMap<&str, usize> = MIN_CHECKS_PER_BACKEND.iter().copied().collect();
    let mut violations: Vec<String> = Vec::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();

    for backend in PASTE_BACKENDS {
        let name = backend.name;
        counts.insert(name, 0);

        for stem in FIXTURES {
            let source = generate(backend.lang, stem);

            for extractor in backend.extractors {
                let (pairs, errors) = match extractor {
                    Extractor::HostPairs {
                        marker,
                        key_at,
                        value_at,
                    } => extract_host_pairs(&source, marker, *key_at, *value_at),
                    Extractor::LuaTable { marker } => extract_lua_tables(&source, marker),
                };
                violations.extend(
                    errors
                        .into_iter()
                        .map(|why| format!("{name} / {stem}: {why}")),
                );

                // Every occurrence is judged, not just the first per key:
                // the same param reaches the runtime through more than
                // one site (Rust and Go emit the HTTP shape into both the
                // event-data table and the form-field map), and those
                // sites were measured to escape differently. Deduplicating
                // would let one correct site vouch for a broken sibling.
                for (key, observed) in pairs {
                    let authored =
                        authored_value_for(&key).expect("filtered to authored keys already");
                    *counts.get_mut(name).expect("counter seeded") += 1;
                    if observed != authored {
                        violations.push(format!(
                            "{name} / {stem}: param {key:?} value changed crossing the \
                             literal boundary\n  authored: {authored:?}\n  observed: {observed:?}",
                        ));
                    }
                }
            }
        }
    }

    // Asserted before the floors: a truncated literal both suppresses the
    // count and explains why, so reporting the count first would replace
    // the diagnosis with its symptom.
    assert!(
        violations.is_empty(),
        "author `<param>` literals did not survive codegen:\n\n{}",
        violations.join("\n\n"),
    );

    let shortfalls: Vec<String> = counts
        .iter()
        .filter_map(|(name, got)| {
            let want = floors
                .get(name)
                .unwrap_or_else(|| panic!("{name} has no floor in MIN_CHECKS_PER_BACKEND"));
            (got < want).then(|| format!("  {name}: checked {got}, floor {want}"))
        })
        .collect();
    assert!(
        shortfalls.is_empty(),
        "fewer `<param>` values were verified than the templates emit — a paste site \
         went unread:\n{}",
        shortfalls.join("\n"),
    );
}
