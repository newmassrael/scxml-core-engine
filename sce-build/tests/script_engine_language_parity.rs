// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The manifest's `script_engine_language` is a claim about the ARTIFACT, so
//! it is measured against the artifact.
//!
//! `needs_script_engine` tells a host it must supply an engine;
//! `script_engine_language` tells it which kind. Until 2026-08-27 the second
//! was a constant — `"lua"`, for every backend — under a comment asserting
//! that "the lowering happens in `sce-build`, before any backend renders, so
//! every language's generated machine evaluates the same Lua". Four backends
//! do work that way. C++ and Kotlin do not: their generated code takes its
//! engine by injection, cannot know at generation time which one arrives, and
//! therefore hands over the author's ECMAScript *source* — which is exactly
//! why each of them carries a runtime rewriter, and why those two rewriters
//! are where the ECMA-262 divergences live. Both default to an ECMAScript
//! engine (`SCE_SCRIPT_ENGINE=quickjs`, `W3CTestBase.DEFAULT_ENGINE="rhino"`),
//! so a host obeying the manifest supplied the wrong engine for the two
//! backends that most needed the right one.
//!
//! `docs/SCE_LUA_TRANSLATION_SEAM.md` carries the per-backend table. What is
//! checked HERE is that the mapping on `Language::script_engine_language`
//! still describes what each backend emits — and that is asked of the emitted
//! text, not of the templates:
//!
//! * A backend that lowers at build time emits the frontend's own Lua, and
//!   `_scxml_truthy(` — the helper that lowering introduces — appears in the
//!   artifact.
//! * A backend that hands over source never emits it.
//!
//! The evidence is that POSITIVE marker rather than the presence of the
//! author's ECMAScript, and the first run of this gate is why: every backend
//! echoes the original expression into a comment beside the guard
//! (`/* W3C SCXML 3.13: cond="..." (lua eval) */` on C11), so searching for
//! the ECMAScript spelling reported C11 — a lowering backend — as carrying
//! source. A scanner that reads comments as code cannot tell a backend's
//! output from its documentation.
//!
//! Reading the artifact rather than grepping the templates for `to_lua_guard`
//! matters: a template could gain the filter at one site and keep source at
//! another, and the half-converted backend would still look converted to a
//! grep. The document's own text is the thing a host would have to evaluate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::{Language, ScriptEngineTarget};
use sce_build::manifest::{
    SCRIPT_ENGINE_LANGUAGES, SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT, SCRIPT_ENGINE_LANGUAGE_LUA,
};

/// A committed document with a guard that survives translation visibly: `&&`
/// becomes `and` and `===` becomes `==`, so the ECMAScript spelling below
/// appears only where the source was passed through. Named rather than
/// generated, so a fixture that moves fails against a written-down answer
/// instead of agreeing with itself. It carries no `<invoke>`, so every
/// backend generates it.
const FIXTURE: &str =
    "integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml";

/// What build-time lowering leaves behind, and a comment cannot.
///
/// `to_lua_guard` wraps a truthiness test in the frontend's own helper, so
/// this substring is present exactly in the artifacts that carry lowered Lua.
/// The author's ECMAScript is echoed into comments on every backend and is
/// therefore useless as the discriminator — see the module note.
const LOWERED_MARKER: &str = "_scxml_truthy(_event.data)";

/// The prose surface: the per-backend table a reader consults.
const SEAM_DOC: &str = "docs/SCE_LUA_TRANSLATION_SEAM.md";
const SEAM_ANCHOR: &str = "sce:lua-translation-seam";
/// The two verdicts its last column may carry.
const SEAM_SOURCE_SIDE: &str = "ECMAScript source";
const SEAM_LOWERED_SIDE: &str = "translated Lua";

const SCHEMA: &str = "schemas/sce-manifest.v1.schema.json";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Generate `FIXTURE` for one backend and return `(every emitted byte, the
/// manifest line)`.
fn emit(lang: &str, tag: &str) -> (String, String) {
    let out = repo_root().join("target").join(tag).join(lang);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("scratch dir");

    let result = Command::new(codegen_bin())
        .args([
            "generate",
            FIXTURE,
            "-l",
            lang,
            "-o",
            out.to_str().expect("utf-8 path"),
            "--no-format",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(
        result.status.success(),
        "`sce-codegen generate -l {lang}` refused {FIXTURE}; this gate needs every backend \
         to emit it.\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let mut text = String::new();
    let mut stack = vec![out.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("scratch dir is readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(body) = std::fs::read_to_string(&path) {
                text.push_str(&body);
                text.push('\n');
            }
        }
    }
    assert!(
        !text.is_empty(),
        "{lang}: generation wrote nothing readable, so there is no artifact to measure"
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let manifest = stdout
        .lines()
        .last()
        .unwrap_or_else(|| panic!("{lang}: no manifest on stdout"))
        .to_string();
    (text, manifest)
}

/// Ask one backend for an explicit target; `None` is a refusal, `Some` is
/// every byte it emitted.
///
/// Unlike [`emit`] this does NOT assert success — a refusal is one of the two
/// answers the case below compares against — and it returns the ARTIFACT,
/// because "did the CLI accept" is not an independent reading of
/// `supports_script_engine_target`: the CLI derives its refusal from that same
/// function, and a gate whose two halves share a source is not a gate.
/// Measured 2026-08-30: with the ECMAScript arm forced to `true`, all twelve
/// combinations were accepted, so an accept/refuse comparison alone had
/// nothing left to disagree with.
fn asks_for(lang: &str, target: &str, tag: &str) -> Option<String> {
    let out = repo_root().join("target").join(tag).join(lang);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("scratch dir");

    let result = Command::new(codegen_bin())
        .args([
            "generate",
            FIXTURE,
            "-l",
            lang,
            "-o",
            out.to_str().expect("utf-8 path"),
            "--script-engine",
            target,
            "--no-format",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    if !result.status.success() {
        return None;
    }

    let mut text = String::new();
    let mut stack = vec![out.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("scratch dir is readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(body) = std::fs::read_to_string(&path) {
                text.push_str(&body);
                text.push('\n');
            }
        }
    }
    assert!(
        !text.is_empty(),
        "{lang}/{target}: generation reported success and wrote nothing readable, so there \
         is no artifact to measure"
    );
    Some(text)
}

/// A target a backend ACCEPTS is a target its artifact is actually in.
///
/// `supports_script_engine_target` is what a host reads before it supplies an
/// engine, so a backend that accepts `--script-engine ecmascript` has to emit
/// the author's source and one that accepts `lua` has to emit lowered Lua. The
/// reading is the ARTIFACT — `LOWERED_MARKER`, what build-time lowering leaves
/// behind and a comment cannot — because the CLI's own refusal is derived from
/// the very predicate under test. An accept/refuse comparison would be that
/// predicate agreeing with itself; the marker is the independent half.
///
/// This case exists because the ECMAScript arm of that function was a flat
/// `false` until 2026-08-30 and nothing here would have faulted it. C++ and
/// Kotlin were answered `true` by the `target == default` line above it, so a
/// CAPABILITY question was being settled by a POLICY — and
/// `ecma262_scoreboard_contract` derives the `runtime-rewriter` path from this
/// answer, so a path that vanished when a default moved would take a
/// divergence list's entries with it while the engine still answered every one
/// of them exactly as before.
///
/// The refusal side is not decoration: four backends lower unconditionally and
/// have no arm that emits the author's source, so a green run is four measured
/// refusals beside eight measured artifacts, never a sweep that found nothing.
#[test]
fn the_targets_a_backend_claims_are_the_targets_it_generates_for() {
    let mut emitted = 0usize;
    let mut refused = 0usize;

    for lang in Language::ALL {
        let name = lang.canonical_name();
        // `ScriptEngineTarget::ALL`, not the two spelled out: this sweep is
        // one of the places a third engine language must reach on the day it
        // lands, and a restated pair here would leave it measured for two
        // targets while the wire admitted three.
        for target in ScriptEngineTarget::ALL.iter().copied() {
            let wire = target.wire_name();
            let claimed = lang.supports_script_engine_target(target);
            let artifact = asks_for(name, wire, "seam-target-parity");
            assert_eq!(
                claimed,
                artifact.is_some(),
                "`{name}` says supports_script_engine_target({wire}) = {claimed} and \
                 `sce-codegen generate -l {name} --script-engine {wire}` {}. The refusal is \
                 the contract a host reads before it supplies an engine.",
                if artifact.is_some() {
                    "succeeded"
                } else {
                    "refused"
                }
            );

            let Some(text) = artifact else {
                refused += 1;
                continue;
            };
            emitted += 1;

            let lowered = text.contains(LOWERED_MARKER);
            let wanted = target == ScriptEngineTarget::Lua;
            assert_eq!(
                lowered,
                wanted,
                "`{name}` accepted `--script-engine {wire}` and emitted an artifact that \
                 {} `{LOWERED_MARKER}`. Accepting a target means emitting FOR it: a backend \
                 whose templates only lower cannot honour the ECMAScript request, and one \
                 that hands over source cannot honour the Lua request — either way the host \
                 supplies the engine the manifest named and the machine speaks the other \
                 language, with no diagnostic anywhere.",
                if lowered { "carries" } else { "does not carry" }
            );
        }
    }

    assert!(
        emitted > 0 && refused > 0,
        "this case measured {emitted} emission(s) and {refused} refusal(s). Both sides have \
         to be non-empty or it is asserting one answer against itself: today four backends \
         refuse the ECMAScript target and two accept it."
    );
}

#[test]
fn every_backend_maps_into_the_wire_vocabulary() {
    let vocabulary: BTreeSet<&str> = SCRIPT_ENGINE_LANGUAGES.iter().copied().collect();
    assert_eq!(
        vocabulary.len(),
        SCRIPT_ENGINE_LANGUAGES.len(),
        "the wire vocabulary lists a spelling twice"
    );
    for lang in Language::ALL {
        assert!(
            vocabulary.contains(lang.script_engine_language()),
            "`{}` maps to `{}`, which is not in the wire vocabulary {:?} — so the manifest \
             would emit a value the schema refuses.",
            lang.canonical_name(),
            lang.script_engine_language(),
            SCRIPT_ENGINE_LANGUAGES
        );
    }
}

#[test]
fn the_schema_admits_exactly_the_wire_vocabulary() {
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(repo_root().join(SCHEMA)).expect("schema"))
            .expect("schema is JSON");
    let declared: BTreeSet<String> = schema
        .pointer("/properties/script_engine_language/enum")
        .and_then(|e| e.as_array())
        .unwrap_or_else(|| panic!("{SCHEMA} declares no enum for script_engine_language"))
        .iter()
        .map(|v| v.as_str().expect("enum entries are strings").to_string())
        .collect();
    let ours: BTreeSet<String> = SCRIPT_ENGINE_LANGUAGES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        declared, ours,
        "the manifest schema's `script_engine_language` enum and the producer's vocabulary \
         disagree. The producer emits what the enum refuses, or the enum promises a value \
         nothing produces — and this field's whole job is telling a host which engine to \
         build."
    );
}

/// The claim, measured on the emitted text of all six.
#[test]
fn the_reported_engine_language_matches_what_the_artifact_carries() {
    let mut source_side = 0usize;
    let mut lowered_side = 0usize;

    for lang in Language::ALL {
        let name = lang.canonical_name();
        let (text, manifest) = emit(name, "script_engine_language_parity");
        let carries_lowered_lua = text.contains(LOWERED_MARKER);
        let reported = lang.script_engine_language();

        if carries_lowered_lua {
            lowered_side += 1;
            assert_eq!(
                reported, SCRIPT_ENGINE_LANGUAGE_LUA,
                "`{name}` emits lowered Lua (`{LOWERED_MARKER}` is in its output), so its host \
                 needs a Lua engine — but the manifest says `{reported}`."
            );
        } else {
            source_side += 1;
            assert_eq!(
                reported, SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT,
                "`{name}` emits no lowered Lua, so it hands the engine the author's ECMAScript \
                 and its host must supply an ECMAScript engine — but the manifest says \
                 `{reported}`. A host obeying that builds a Lua engine and hands it JavaScript."
            );
        }

        // The manifest is the surface a host actually reads, so the value is
        // read back off it rather than trusted from the mapping alone.
        let parsed: serde_json::Value = serde_json::from_str(&manifest)
            .unwrap_or_else(|e| panic!("{name}: manifest is not JSON ({e}): {manifest}"));
        assert_eq!(
            parsed
                .get("script_engine_language")
                .and_then(|v| v.as_str()),
            Some(reported),
            "`{name}`: the emitted manifest does not carry the engine language this backend \
             needs.\nmanifest: {manifest}"
        );
    }

    // Both sides must be populated. A sweep that put every backend on one
    // side would pass every assertion above by asking the same question six
    // times, and the split IS the finding this gate exists to hold.
    assert!(
        source_side >= 1 && lowered_side >= 1,
        "the six backends landed {source_side} on the source side and {lowered_side} on the \
         lowered side. Both sides existing is the fact this gate measures; if a change moved \
         them all to one side, the mapping and the seam document both need rewriting rather \
         than this floor relaxing."
    );
}

/// The third surface: the table a reader consults.
///
/// The field is derived from the templates and held to the artifact by the
/// case above, which leaves one way for the three to disagree — the prose
/// going stale while both machine surfaces move together. A reader picking an
/// engine reads the table, not the code, so it is checked rather than trusted.
#[test]
fn the_seam_table_and_the_derived_answer_agree() {
    let doc = std::fs::read_to_string(repo_root().join(SEAM_DOC))
        .unwrap_or_else(|e| panic!("{SEAM_DOC} is readable: {e}"));
    let at = doc.find(SEAM_ANCHOR).unwrap_or_else(|| {
        panic!(
            "{SEAM_DOC} carries no `{SEAM_ANCHOR}` anchor. That table is the surface a reader \
             consults before choosing an engine; without the anchor this case reads nothing \
             and would pass by reading nothing."
        )
    });

    let mut rows: Vec<(String, &'static str)> = Vec::new();
    let mut header_seen = false;
    for line in doc[at..].lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            if rows.is_empty() && !header_seen {
                continue;
            }
            if trimmed.starts_with('|') {
                continue;
            }
            if rows.is_empty() {
                continue;
            }
            break;
        }
        let cells: Vec<&str> = trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 4 || !cells[0].starts_with('`') {
            continue;
        }
        if !header_seen {
            assert_eq!(
                cells[0], "`--lang`",
                "the seam table's first column is no longer the `--lang` spelling. It is read \
                 positionally and compared with `Language::canonical_name`, so a display name \
                 there has to fail here rather than be matched by guesswork."
            );
            header_seen = true;
            continue;
        }
        let verdict = cells[cells.len() - 1];
        let side = if verdict.contains(SEAM_SOURCE_SIDE) {
            SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT
        } else if verdict.contains(SEAM_LOWERED_SIDE) {
            SCRIPT_ENGINE_LANGUAGE_LUA
        } else {
            panic!(
                "the seam table's row for {} says '{verdict}'. The column has two readings — \
                 `{SEAM_SOURCE_SIDE}` or `{SEAM_LOWERED_SIDE}` — because it answers which \
                 language the engine must evaluate, and a third phrase is a claim nothing \
                 checks.",
                cells[0]
            );
        };
        rows.push((cells[0].trim_matches('`').to_string(), side));
    }

    assert_eq!(
        rows.len(),
        Language::ALL.len(),
        "the seam table has {} row(s) for {} backends. Every backend needs a row: a missing \
         one is a backend whose engine a reader has to infer.\nrows: {rows:?}",
        rows.len(),
        Language::ALL.len()
    );

    for lang in Language::ALL {
        let name = lang.canonical_name();
        let (_, documented) = rows
            .iter()
            .find(|(row, _)| row == name)
            .unwrap_or_else(|| panic!("the seam table names no row for `{name}`"));
        assert_eq!(
            *documented,
            lang.script_engine_language(),
            "the seam table says `{name}` needs a `{documented}` engine and the code derives \
             `{}` from the templates. The table is what a reader picks an engine from, so a \
             stale row sends them to the wrong one.",
            lang.script_engine_language()
        );
    }
}
