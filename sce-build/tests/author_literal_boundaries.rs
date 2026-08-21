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
// Backends absent from the *value* gate, each measured rather than
// assumed:
//   c11    — never pastes a static value; it assembles Lua source whose
//            values are evaluated at runtime, so there is no authored
//            value to compare. Its param *names* are covered by the
//            second test in this file.
//   python — already correct, and the reference the others were brought
//            up to: `py_string_literal` is applied at every param slot,
//            name and value alike, and the emitted module compiles.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sce_build::compile_scxml_lang;
use sce_build::generator::Language;

/// A document to compile, and what has to be in scope for its paste
/// sites to render.
struct Fixture {
    /// Stem of the SCXML file, resolved by [`fixture_path`].
    stem: &'static str,
    /// Directory under `tests/fixtures/` holding it.
    dir: &'static str,
    /// Deploy document that must be in scope, as a file name in the same
    /// directory. `None` compiles the document alone.
    ///
    /// A deploy is not decoration here: it is the only way to reach the
    /// emission blocks a backend selects on topology. Without one,
    /// `is_remote_invoke_target` is false by construction and the mesh
    /// send site cannot render, however the document is written.
    deploy: Option<&'static str>,
}

/// Documents the value gate compiles.
///
/// The first pair sits under `codegen_smoke/` and is shared with that
/// gate, which compiles the same text instead of reading it. The pair
/// exists because `needs_script_engine` selects mutually exclusive
/// emission blocks: one document cannot reach both, so one document
/// cannot prove both.
///
/// The third is deploy-scoped and not shared: `codegen_smoke` compiles
/// single documents, so registering it there would exercise the local
/// send block under a mesh fixture's name — the opposite of what it is
/// for. Its C++ output goes unchecked by a compiler as a result, which
/// is a real gap and a narrow one: the block differs from its already-
/// compiled sibling in the call it makes, not in how it pastes literals.
const FIXTURES: &[Fixture] = &[
    Fixture {
        stem: "send_param_adversarial_literals",
        dir: "codegen_smoke",
        deploy: None,
    },
    Fixture {
        stem: "send_param_adversarial_literals_scripted",
        dir: "codegen_smoke",
        deploy: None,
    },
    Fixture {
        stem: "mesh_worker",
        dir: "author_literal_mesh",
        deploy: Some("deploy.yaml"),
    },
];

/// `<send>` shape prefixes used by the fixtures' param names.
///
/// `meshparent` is the deploy-scoped send-to-parent site; `parent` is the
/// single-document one. They are separate prefixes because they are
/// separate paste sites, and a shared prefix would let either one's count
/// stand in for the other.
const SHAPES: &[&str] = &[
    "internal",
    "parent",
    "meshparent",
    "pdelay",
    "http",
    "httpmix",
    "texpr",
    "mix",
    "delay",
    "plain",
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
    /// A whole JSON payload carried inside one host string literal —
    /// §scxml-B-2-9's wire, which the receiver parses.
    WirePayload { marker: &'static str },
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
            Extractor::WirePayload {
                marker: "let event_data",
            },
            Extractor::HostPairs {
                marker: "http_params.entry(",
                key_at: 0,
                value_at: 1,
            },
            // Runtime-assembled event data: the params are collected as
            // typed values and serialised by the runtime helper, so the
            // authored name and value are literals 0 and 1 of the line that
            // enters one.
            Extractor::HostPairs {
                marker: "wire_params.entry(",
                key_at: 0,
                value_at: 1,
            },
        ],
    },
    Backend {
        lang: Language::Go,
        name: "go",
        extractors: &[
            Extractor::WirePayload {
                marker: "eventDataStr",
            },
            Extractor::HostPairs {
                marker: "httpParams[",
                key_at: 0,
                value_at: 2,
            },
            // Same shape as rust's `wire_params.entry(`: a struct literal
            // carrying the authored name and value into the runtime
            // serialiser.
            Extractor::HostPairs {
                marker: "sce.EventDataParam{",
                key_at: 0,
                value_at: 1,
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
            // One marker for all four send shapes: they used to write the
            // map directly (`paramsP["k"] = v`) and now go through
            // `putParam(paramsP, "k", v)`, which is what lets a repeated
            // name become an Array instead of overwriting.
            Extractor::HostPairs {
                marker: "putParam(params",
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
///
/// Rust and Go dropped from 126 to 119 on 2026-08-21, and the seven sites
/// each lost were duplicates rather than coverage: their BasicHTTP and
/// host-served arms used to re-collect `<param>` from the data model after
/// the send-time evaluation had already done it, so the same authored
/// literal was pasted twice per document. §scxml-6.2.3 evaluates a
/// `<send>`'s arguments once; the arms now render that one evaluation, so
/// there is one paste site where there were two. Lowering a floor is the
/// one edit this constant invites suspicion of — the difference is that
/// the sites are gone from the templates, not merely unread by the scan,
/// which `send_namelist_over_http` witnesses from the other side.
const MIN_CHECKS_PER_BACKEND: &[(&str, usize)] =
    &[("cpp", 63), ("rust", 119), ("go", 119), ("kotlin", 105)];

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture_path(dir: &str, stem: &str) -> PathBuf {
    fixtures_root().join(dir).join(format!("{stem}.scxml"))
}

/// Every generated file for `lang`, concatenated. Backends split output
/// across several files (cpp emits `_sm.h` + `_sm.inl`); the paste site
/// may live in any of them.
///
/// Deploy-scoped fixtures go through `compile_scxml_lang_with_deploy`,
/// which applies the same model mutations the CLI applies, through the
/// same function. That sharing is not tidiness: passing the deploy to a
/// call that only *reads* it is not enough — `is_remote_invoke_target` is
/// set by the mutation pass, and without it the mesh send site does not
/// render at all. A first attempt here did exactly that and the site
/// count still rose by seven, because other sites in the same fixture
/// supplied them; only mutating the site and watching this gate stay
/// green revealed it.
fn generate(lang: Language, fixture: &Fixture) -> String {
    let stem = fixture.stem;
    let path = fixture_path(fixture.dir, stem);
    let path = path.to_str().expect("fixture path is UTF-8");
    let template_dir = sce_build::find_template_dir_for(lang);

    let output = match fixture.deploy {
        None => compile_scxml_lang(path, &template_dir, lang),
        Some(deploy_name) => {
            let deploy_path = fixtures_root().join(fixture.dir).join(deploy_name);
            sce_build::compile_scxml_lang_with_deploy(path, &template_dir, lang, &deploy_path, None)
                .map_err(|e| e.to_string())
        }
    }
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

/// Extract every payload a backend embedded in a host literal, undo the
/// host escaping, then read each one the way the RECEIVER does.
///
/// Per-payload this is all-or-nothing by nature: the params of one `<send>`
/// share a single document, so one bad value makes the whole thing
/// unparseable and there is no per-param verdict to give. The error carries
/// the emitted text so a failing run shows what was written.
fn extract_wire_payloads(source: &str, marker: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut errors: Vec<String> = Vec::new();
    let mut found = Vec::new();
    let lua = mlua::Lua::new();
    // §scxml-B-2-9: the payload is JSON, so it is read back with the shared
    // `JSON.parse` rather than by evaluating it. That is not only a change of
    // format — it is the claim this test now makes. The payload used to be
    // `_scxml_params({"k", v}, …)`, Lua SOURCE, and reading it back meant
    // loading the shared semantics file and RUNNING what the sender wrote;
    // the receiving engines did the same, which is how an event payload
    // became executable. Using the production reader rather than a
    // reimplementation is what keeps this gate measuring what ships.
    lua.load(sce_build::filters::JSON_BUILTINS_LUA)
        .exec()
        .expect("the shared JSON file loads");

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
        let wire = match unescape_host(&raw) {
            Ok(source) => source,
            Err(why) => {
                errors.push(format!("host literal (raw {raw:?}): {why}"));
                continue;
            }
        };
        let trimmed = wire.trim();
        if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
            continue;
        }

        // Handed over as a value, not spliced into the chunk: a test that
        // pasted the payload into Lua source would be re-introducing the
        // boundary it exists to check.
        lua.globals()
            .set("_wire", trimmed)
            .expect("the payload binds");
        let parsed: Option<mlua::Table> = match lua
            // `_parse_document` rather than `JSON.parse`: the author-facing
            // entry point answers with the value alone (ECMA-262 gives it one
            // return), and this has to tell a refusal from a JSON `null`.
            .load("local v, ok = JSON._parse_document(_wire); if ok then return v end return nil")
            .eval()
        {
            Ok(parsed) => parsed,
            Err(e) => {
                errors.push(format!(
                    "the shared JSON reader raised: {e}\n  wire was: {wire}"
                ));
                continue;
            }
        };
        let Some(table) = parsed else {
            errors.push(format!(
                "the emitted payload is not JSON — host source may still compile\n  \
                 wire was: {wire}"
            ));
            continue;
        };
        for pair in table.pairs::<String, String>() {
            match pair {
                Ok((k, v)) => {
                    if authored_value_for(&k).is_some() {
                        found.push((k, v));
                    }
                }
                Err(e) => errors.push(format!("payload entry is not string→string: {e}")),
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

        for fixture in FIXTURES {
            let stem = fixture.stem;
            let source = generate(backend.lang, fixture);

            for extractor in backend.extractors {
                let (pairs, errors) = match extractor {
                    Extractor::HostPairs {
                        marker,
                        key_at,
                        value_at,
                    } => extract_host_pairs(&source, marker, *key_at, *value_at),
                    Extractor::WirePayload { marker } => extract_wire_payloads(&source, marker),
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

// ── `<donedata>` param names ────────────────────────────────────────
//
// A separate axis from the `<send>` values above, and separate because
// its failure mode is the mirror image.
//
// Three backends lower `<donedata>` by assembling *Lua source* — a table
// assignment whose key is the author's param name — and embedding that
// source in a host string literal. The value is an expression evaluated
// at runtime, so there is no authored value to compare; the name is the
// only author text that crosses the boundary, and it crosses two.
//
// C11 additionally puts that name in Lua *index* position
// (`_pending_donedata["k"]`). It used to sit in field position
// (`_pending_donedata.k`), where no escaping can help because an
// arbitrary string is not an identifier — the fix was index syntax, and
// this gate is what holds it there.
//
// Measured, not assumed: mutating away the Lua layer while leaving the
// host layer in place keeps `codegen_smoke` fully green on Rust and Go,
// because the host source still compiles and the Lua it carries is never
// handed to a compiler. Only parsing that Lua catches it.
//
// Coverage, measured per site by mutating each one individually — all
// five Lua-assembling sites go red: rust donedata, go donedata, and all
// three C11 macros (donedata static-expr, send dynamic-param, and that
// one's empty-string error fallback). The C11 rows also go red when
// reverted to field syntax, which is the regression guard for the defect
// that motivated the index-syntax rewrite.
//
// Two things had to be true before that held, and neither was obvious:
//
//   count occurrences, not distinct names — C11 writes the same key from
//     two lines (`= _v;` and its `= '';` fallback). Deduplicating let an
//     intact line vouch for a mutated sibling.
//
//   check both directions — asserting only that every authored name
//     appears misses the case that matters, because a truncated name is
//     an *extra* key, not a missing one. An unescaped quote cut
//     `delay_odd"name` down to `delay_odd` while the correct spelling
//     still arrived from the other line, and presence-only checking
//     stayed green.

/// Fixture whose `<donedata>` params carry literal-breaking names.
const DONEDATA_FIXTURE: &str = "donedata_adversarial_literals";

/// Sites that assemble Lua source inside a host literal, as
/// `(language, label, fixture, line marker, param-name prefix, floor)`.
///
/// The C11 backend appears several times because separate emission
/// paths each assemble Lua source. Two macros write to the same
/// `_pending_donedata` table — one lowers `<donedata>` params, the other
/// a `<send>` param evaluated at runtime — and they share a marker but
/// not a call path, so covering one proves nothing about the other. The
/// `<send target="#_parent">` path builds its own table constructor in a
/// C buffer instead and needs its own marker; it was writing names in
/// Lua *field* position (`name = v`, where no escaping can help) long
/// after the donedata path had moved to index syntax, and no row here
/// looked at it.
const LUA_KEY_SITES: &[(Language, &str, &str, &str, &str, usize)] = &[
    (
        Language::Rust,
        "rust/donedata",
        DONEDATA_FIXTURE,
        "let mut part = String::from(",
        "dd_",
        7,
    ),
    (
        Language::Go,
        "go/donedata",
        DONEDATA_FIXTURE,
        "jsonParts = append(jsonParts,",
        "dd_",
        7,
    ),
    (
        Language::C11,
        "c11/donedata",
        DONEDATA_FIXTURE,
        "_pending_donedata[",
        "dd_",
        7,
    ),
    (
        Language::C11,
        "c11/send-dynamic-param",
        "send_param_adversarial_literals_scripted",
        "_pending_donedata[",
        "delay_",
        14,
    ),
    // `<send target="#_parent">` with params. Both fixtures reach it:
    // the payload is assembled from a C buffer either way, so unlike the
    // sites above this one does not branch on `needs_script_engine`.
    // Kept as two rows rather than one because that independence is the
    // claim — if either fixture stops reaching the site, only the row
    // for that fixture goes red and says which.
    //
    // The marker is the codegen-time JSON literal (`*_lit = `) rather than
    // the per-name buffer append it used to be: params that all fold are
    // serialised whole at build time now, which is what lets a machine with
    // no script engine still produce a readable payload. Both fixtures take
    // that arm — measured, the "scripted" one included, because its
    // parent-send params are literals even where its others are not.
    (
        Language::C11,
        "c11/parent-send-param",
        "send_param_adversarial_literals",
        "*_lit = ",
        "parent_",
        7,
    ),
    (
        Language::C11,
        "c11/parent-send-param-scripted",
        "send_param_adversarial_literals_scripted",
        "*_lit = ",
        "parent_",
        7,
    ),
];

/// Pull `["<key>"]` keys out of Lua source embedded in host literals on
/// lines carrying `marker`, evaluating each key with the real Lua parser.
fn extract_lua_keys(source: &str, marker: &str) -> (Vec<String>, Vec<String>) {
    let mut keys = Vec::new();
    let mut errors = Vec::new();
    let lua = mlua::Lua::new();

    for line in source.lines() {
        if !line.contains(marker) {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        for raw in literals_after(&chars, 0) {
            let lua_source = match unescape_host(&raw) {
                Ok(source) => source,
                Err(why) => {
                    errors.push(format!("host literal (raw {raw:?}): {why}"));
                    continue;
                }
            };
            let mut rest = lua_source.as_str();
            // Three spellings, because a `<param>` name reaches the layer
            // below in three shapes: the Lua index `["name"] = v` where the
            // emitter still writes into a table (C11's `_pending_donedata`),
            // the pair `{"name", v}` where it writes `_scxml_params(...)`,
            // and the JSON member `"name":v` on the §scxml-B-2-9 wire that
            // `<send>` and `<donedata>` now write. All three put the name in
            // a quoted literal, which is all this scan depends on; each
            // candidate below is the index OF THE QUOTE.
            let next_key = |s: &str| {
                [
                    s.find("[\"").map(|i| i + 1),
                    s.find("{\"").map(|i| i + 1),
                    // A JSON payload opens with its first key, and a member
                    // after the first follows a comma.
                    if s.starts_with('"') { Some(0) } else { None },
                    s.find(",\"").map(|i| i + 1),
                ]
                .into_iter()
                .flatten()
                .min()
            };
            while let Some(at) = next_key(rest) {
                let tail: Vec<char> = rest[at..].chars().collect();
                match scan_literal(&tail, 0) {
                    Some((raw_key, _)) => {
                        // The key is a Lua literal, so Lua decides what it
                        // means. Re-implementing that decision here would
                        // let the check drift from the grammar it guards.
                        match lua.load(format!("return \"{raw_key}\"")).eval::<String>() {
                            Ok(key) => keys.push(key),
                            Err(e) => errors.push(format!(
                                "emitted Lua key does not parse — host source still \
                                 compiles: {e}\n  Lua key literal was: \"{raw_key}\""
                            )),
                        }
                    }
                    None => errors.push(format!(
                        "unterminated Lua string in emitted source: {lua_source}"
                    )),
                }
                rest = &rest[at + 1..];
            }
        }
    }
    (keys, errors)
}

/// Every `<donedata>` param name must arrive at the Lua layer intact.
#[test]
fn donedata_param_names_survive_the_embedded_lua_boundary() {
    let mut violations: Vec<String> = Vec::new();

    for (lang, name, fixture, marker, prefix, floor) in LUA_KEY_SITES {
        let expected: Vec<String> = VALUES
            .iter()
            .map(|(suffix, _)| format!("{prefix}{suffix}"))
            .collect();

        // Every Lua-key site lives in a single-document fixture; none of
        // them is topology-dependent.
        let source = generate(
            *lang,
            &Fixture {
                stem: fixture,
                dir: "codegen_smoke",
                deploy: None,
            },
        );
        let (keys, errors) = extract_lua_keys(&source, marker);
        violations.extend(errors.into_iter().map(|why| format!("{name}: {why}")));

        // Occurrences, not distinct names: a backend can write the same
        // key from more than one line (C11's `= _v;` and its `= '';`
        // error fallback both do). Deduplicating lets one intact line
        // vouch for a broken sibling — measured, after a mutation to a
        // single line left this gate green.
        let authored: Vec<&String> = keys.iter().filter(|k| k.starts_with(*prefix)).collect();
        let distinct: BTreeMap<&String, ()> = authored.iter().map(|k| (*k, ())).collect();

        for want in &expected {
            if !distinct.contains_key(want) {
                violations.push(format!(
                    "{name}: param {want:?} did not survive to the Lua layer\n  \
                     names that did: {:?}",
                    distinct.keys().collect::<Vec<_>>(),
                ));
            }
        }

        // The converse, and the half that matters more: a name the author
        // never wrote means a name was truncated on its way here. Checking
        // only for presence lets a correct line vouch for a broken one —
        // an unescaped quote cut `delay_odd"name` down to `delay_odd`,
        // both keys were emitted, and presence-only checking stayed green.
        for got in distinct.keys() {
            if !expected.contains(got) {
                violations.push(format!(
                    "{name}: emitted param name {got:?} is not one the fixture wrote — \
                     an unescaped character truncated it\n  authored: {expected:?}",
                ));
            }
        }

        // Floor for the same reason as the send gate: a marker that stops
        // matching would otherwise read every bit as green as a correct
        // backend.
        if authored.len() < *floor {
            violations.push(format!(
                "{name}: recovered {} param-name occurrences, floor {floor} — a paste \
                 site moved or stopped emitting",
                authored.len(),
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "`<donedata>` param names did not survive the embedded Lua boundary:\n\n{}",
        violations.join("\n\n"),
    );
}
