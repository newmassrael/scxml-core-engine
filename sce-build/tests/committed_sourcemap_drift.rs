// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Committed-sourcemap drift gate.
//
// The `sce_sourcemap.json` sidecars checked into the backend test
// trees are generated artifacts, but nothing compared them against the
// generator that produces them:
//
//   - `sce-codegen verify` reads the drift header, and a JSON sidecar
//     carries no comment syntax to put one in.
//   - `b9_drift_detection::committed_trees_carry_a_pinned_generated_at`
//     walks the same trees but only considers files with a
//     `// generated-at:` line, so every sidecar falls through.
//   - The sidecar's own `source_hash` / `template_hash` cover the
//     *inputs*. A change in what the emitter writes for unchanged
//     inputs — new field, dropped field, different value — moves
//     neither hash.
//
// Measured consequence, and the reason this gate exists: the emitter
// was changed to populate `SourceSymbol::event`, which had been fed by
// a stub returning `None`. Every one of the 404 committed sidecars was
// then stale, and the full `sce-build` suite still passed with zero
// failures.
//
// The check is a regeneration comparison because nothing weaker can
// see that class of drift. It compares the `symbols` table only:
// `source_hash` / `template_hash` depend on the input directory the
// caller names and belong to the drift-hash axis, which has its own
// gate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Trees holding committed backend output.
const COMMITTED_TREES: &[&str] = &[
    "backends/rust/tests/src/generated",
    "backends/rust/tests/src/integration",
    "backends/kotlin/tests/src/main/kotlin/com/sce/generated",
    "backends/go/tests/generated",
    "backends/python/tests/integration",
];

/// Directories searched for the SCXML a sidecar traces back to.
///
/// Deliberately narrower than "every `.scxml` in the repo": the
/// template-parity fixture tree reuses names like `main.scxml` across
/// its cases, and a basename index over it would be ambiguous. Over
/// these three roots the index is one-to-one, which
/// [`scxml_index_is_unambiguous`] asserts rather than assumes.
const SCXML_ROOTS: &[&str] = &[
    "resources",
    "integration_resources",
    "sce-build/tests/fixtures",
];

/// Paths git tracks, as repo-relative strings.
///
/// The index is built from tracked files only. `resources/` also holds
/// build products — a W3C case with child machines materialises
/// `resources/<n>a/test<n>a.scxml` next to the tracked
/// `resources/<n>/test<n>a.scxml`, gitignored and identical in
/// basename. Indexing the working tree instead of the tracked set
/// makes those names ambiguous and, worse, lets the gate regenerate
/// from a build product rather than from a source document.
fn tracked_paths() -> std::collections::BTreeSet<String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Lower bound on the sidecars this gate must find. A walk that finds
/// nothing would satisfy every per-file assertion vacuously.
const MIN_SIDECARS: usize = 400;

/// Sidecars git tracks under [`COMMITTED_TREES`].
///
/// Tracked, not merely present: the same trees accumulate untracked
/// build output, and a sidecar that is not committed is not a
/// committed artifact this gate has anything to say about.
fn committed_sidecars() -> Vec<PathBuf> {
    let root = repo_root();
    tracked_paths()
        .iter()
        .filter(|rel| rel.ends_with("/sce_sourcemap.json"))
        .filter(|rel| {
            COMMITTED_TREES
                .iter()
                .any(|tree| rel.starts_with(&format!("{tree}/")))
        })
        .map(|rel| root.join(rel))
        .collect()
}

/// Basename → path for every tracked SCXML under [`SCXML_ROOTS`].
fn scxml_index() -> BTreeMap<String, Vec<PathBuf>> {
    let root = repo_root();
    let tracked = tracked_paths();
    let mut idx: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for rel in &tracked {
        if !rel.ends_with(".scxml") {
            continue;
        }
        if !SCXML_ROOTS
            .iter()
            .any(|r| rel.starts_with(&format!("{r}/")))
        {
            continue;
        }
        let path = root.join(rel);
        let name = Path::new(rel)
            .file_name()
            .and_then(|f| f.to_str())
            .expect("scxml path has a filename")
            .to_string();
        idx.entry(name).or_default().push(path);
    }
    idx
}

/// The index this gate resolves through must be one-to-one.
///
/// If a basename ever becomes ambiguous, the gate would silently
/// regenerate from whichever path sorted first and compare a sidecar
/// against the wrong document.
#[test]
fn scxml_index_is_unambiguous() {
    let idx = scxml_index();
    let ambiguous: Vec<String> = idx
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(name, paths)| format!("{name} -> {paths:?}"))
        .collect();
    assert!(
        ambiguous.is_empty(),
        "SCXML basenames must be unique across {SCXML_ROOTS:?}; ambiguous:\n  {}",
        ambiguous.join("\n  "),
    );
    assert!(
        idx.len() >= 100,
        "index found only {} documents; the roots it walks moved",
        idx.len(),
    );
}

/// `scxml_file` values differ by the path the generator was invoked
/// with, which is not what this gate is about. Compare on basenames.
fn normalise(symbols: &serde_json::Value) -> BTreeMap<String, serde_json::Value> {
    symbols
        .as_object()
        .expect("symbols is an object")
        .iter()
        .map(|(name, entry)| {
            let mut entry = entry.clone();
            if let Some(file) = entry.get("scxml_file").and_then(|v| v.as_str()) {
                let base = Path::new(file)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(file)
                    .to_string();
                entry["scxml_file"] = serde_json::Value::String(base);
            }
            (name.clone(), entry)
        })
        .collect()
}

/// Every committed sidecar equals what the current generator emits.
///
/// One backend is enough: the sidecar is backend-invariant by
/// contract, pinned by
/// `sourcemap_addr2sce::sourcemap_byte_identity_across_backends`. This
/// gate depends on that one and would otherwise have to regenerate
/// each sidecar six times to say the same thing.
#[test]
fn committed_sourcemaps_match_regeneration() {
    let sidecars = committed_sidecars();
    assert!(
        sidecars.len() >= MIN_SIDECARS,
        "found only {} committed sidecars; expected at least {MIN_SIDECARS}. \
         A walk that finds nothing certifies nothing.",
        sidecars.len(),
    );

    let idx = scxml_index();
    let root = repo_root();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("committed-sourcemap-drift-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("create scratch dir");

    let mut stale: Vec<String> = Vec::new();
    let mut compared = 0usize;

    for sidecar in &sidecars {
        let committed: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(sidecar).expect("read committed sidecar"),
        )
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", sidecar.display()));

        // A sidecar covers every machine emitted into its directory,
        // so it may name several documents: an authored parent, each
        // synth-invoke child materialised beside it, and any external
        // `src=` child. Rather than guess which one is "the parent" —
        // the naming rules differ per tree and got this wrong twice —
        // regenerate from every document the sidecar names and compare
        // against the union. Regenerating a parent re-emits its
        // children, so the union is idempotent over the overlap.
        let sources: std::collections::BTreeSet<String> = committed["symbols"]
            .as_object()
            .expect("symbols object")
            .values()
            .map(|s| {
                Path::new(s["scxml_file"].as_str().expect("scxml_file"))
                    .file_name()
                    .and_then(|f| f.to_str())
                    .expect("basename")
                    .to_string()
            })
            .collect();

        let mut got: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        let mut failed = false;
        for (i, source_name) in sources.iter().enumerate() {
            let candidates = idx.get(source_name).unwrap_or_else(|| {
                panic!(
                    "{} traces back to {source_name}, which is not under {SCXML_ROOTS:?}",
                    sidecar.display(),
                )
            });
            let source = &candidates[0];
            let out_dir = scratch.join(format!("case-{compared}-{i}"));
            std::fs::create_dir_all(&out_dir).expect("create case dir");
            let run = Command::new(sce_codegen_bin())
                .arg("generate")
                .arg(source)
                .arg("-l")
                .arg("rust")
                .arg("-o")
                .arg(&out_dir)
                .current_dir(&root)
                .output()
                .expect("invoke sce-codegen");
            if !run.status.success() {
                stale.push(format!(
                    "{}: regeneration from {} failed: {}",
                    sidecar.display(),
                    source.display(),
                    String::from_utf8_lossy(&run.stderr).trim(),
                ));
                failed = true;
                break;
            }
            // A committed sidecar whose regeneration emits none is
            // drift too — the strongest form of it — so it is reported
            // rather than skipped.
            let Ok(text) = std::fs::read_to_string(out_dir.join("sce_sourcemap.json")) else {
                stale.push(format!(
                    "{}: regenerating from {} emitted no sidecar at all",
                    sidecar.display(),
                    source.display(),
                ));
                failed = true;
                break;
            };
            let regenerated: serde_json::Value =
                serde_json::from_str(&text).expect("regenerated sidecar is JSON");
            got.extend(normalise(&regenerated["symbols"]));
            let _ = std::fs::remove_dir_all(&out_dir);
        }
        compared += 1;
        if failed {
            continue;
        }

        let want = normalise(&committed["symbols"]);
        if want != got {
            let missing: Vec<&String> = want.keys().filter(|k| !got.contains_key(*k)).collect();
            let extra: Vec<&String> = got.keys().filter(|k| !want.contains_key(*k)).collect();
            let changed: Vec<&String> = want
                .keys()
                .filter(|k| got.get(*k).is_some_and(|v| v != &want[*k]))
                .collect();
            stale.push(format!(
                "{}: symbols differ (missing {missing:?}, extra {extra:?}, changed {changed:?})",
                sidecar.display(),
            ));
        }
    }

    let _ = std::fs::remove_dir_all(&scratch);

    assert!(
        stale.is_empty(),
        "{} of {compared} committed sourcemaps disagree with the generator. \
         Re-run `scripts/regen_all_committed_trees.sh` and commit the result.\n  {}",
        stale.len(),
        stale
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  "),
    );
    assert!(
        compared >= MIN_SIDECARS,
        "compared only {compared} sidecars; expected at least {MIN_SIDECARS}",
    );
}

/// SCXML line numbers named by the in-source markers in `text`.
///
/// Three spellings, one meaning — the backends differ because `#line`
/// and `//line` are understood by the C and Go toolchains while the
/// rest can only carry a comment:
///   `// SCE-MAP: file.scxml:12`  ·  `#line 12 "file.scxml"`  ·
///   `//line file.scxml:12`
fn marker_lines(text: &str) -> std::collections::BTreeSet<u32> {
    let mut out = std::collections::BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.split("SCE-MAP: ").nth(1).or_else(|| {
            line.strip_prefix("//line ")
                .filter(|_| !line.contains("SCE-MAP"))
        }) {
            // `<file>:<start>[-<end>][ (symbol=…)]` — take the number
            // right after the last colon of the file:line pair.
            let head = rest.split_whitespace().next().unwrap_or("");
            if let Some((_, tail)) = head.rsplit_once(':') {
                let start = tail.split('-').next().unwrap_or("");
                if let Ok(n) = start.trim_end_matches('"').parse::<u32>() {
                    out.insert(n);
                }
            }
        } else if let Some(rest) = line.strip_prefix("#line ") {
            if let Ok(n) = rest.split_whitespace().next().unwrap_or("").parse::<u32>() {
                out.insert(n);
            }
        }
    }
    out
}

/// Lower bound on the directories this gate must judge.
const MIN_MARKER_DIRS: usize = 100;

/// In-source markers must distinguish the states they sit in.
///
/// The markers exist so a listing-only context — a JTAG hard-fault
/// trace, a panic backtrace — can be walked back to SCXML without the
/// sidecar. A marker that names the same line in every function cannot
/// do that, and `validate_emitted_files_have_markers` cannot tell:
/// it checks that the string `SCE-MAP:` occurs, which a file naming
/// one location ten times satisfies exactly as well as a file naming
/// ten.
///
/// Measured before this gate existed: every committed tree carried a
/// single distinct marker value per file, because every call site
/// passed `model.source_location` — the document root — rather than
/// the location of the state or transition it sat in.
///
/// The sidecar beside the emitted files says how many distinct source
/// lines that document actually has, which is what makes this a
/// comparison rather than a magic number.
#[test]
fn committed_markers_distinguish_the_states_they_sit_in() {
    let sidecars = committed_sidecars();
    assert!(
        sidecars.len() >= MIN_SIDECARS,
        "found only {} committed sidecars; expected at least {MIN_SIDECARS}",
        sidecars.len(),
    );

    let mut flat: Vec<String> = Vec::new();
    let mut judged = 0usize;

    for sidecar in &sidecars {
        let dir = sidecar.parent().expect("sidecar has a parent directory");
        let map: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(sidecar).expect("read sidecar"))
                .expect("sidecar is JSON");
        let want: std::collections::BTreeSet<u64> = map["symbols"]
            .as_object()
            .expect("symbols object")
            .values()
            .filter_map(|s| s["line_range"][0].as_u64())
            .collect();
        // A document whose symbols all start on one line has nothing
        // for a marker to distinguish.
        if want.len() < 2 {
            continue;
        }

        let mut got = std::collections::BTreeSet::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_source = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "rs" | "kt" | "go" | "py" | "c" | "h" | "cpp"));
            if !is_source {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                got.extend(marker_lines(&text));
            }
        }
        judged += 1;
        if got.len() < 2 {
            flat.push(format!(
                "{}: {} distinct source lines in the sidecar but {} distinct \
                 marker value(s) in the emitted files",
                dir.display(),
                want.len(),
                got.len(),
            ));
        }
    }

    assert!(
        flat.is_empty(),
        "{} of {judged} emitted directories carry markers that cannot tell \
         their states apart. The call sites must pass the state's or \
         transition's own `source_location`, not the document root:\n  {}",
        flat.len(),
        flat.iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  "),
    );
    assert!(
        judged >= MIN_MARKER_DIRS,
        "judged only {judged} directories; expected at least \
         {MIN_MARKER_DIRS}. A gate that judges nothing certifies nothing.",
    );
}

/// The published sourcemap schema, compiled as draft-07.
fn sourcemap_schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../../schemas/sce-sourcemap.v1.schema.json"))
        .expect("sourcemap schema is valid JSON")
}

/// Schema violations for one sidecar document, as message strings.
fn sourcemap_schema_violations(instance: &serde_json::Value) -> Vec<String> {
    let schema_value = sourcemap_schema();
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("sourcemap schema compiles as draft-07");
    // Bound to a local so the `Result`'s temporary — which carries the
    // borrow of `validator` — drops before `validator` does.
    let msgs: Vec<String> = match validator.validate(instance) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    msgs
}

/// Every committed sidecar is valid against
/// `schemas/sce-sourcemap.v1.schema.json`.
///
/// Nothing else asserts this. [`committed_sourcemaps_match_regeneration`]
/// compares committed bytes against the generator, so the two sides
/// are free to drift away from the schema together and stay green.
/// The guards in `forge::sourcemap` compare the schema *file* against
/// Rust constants — the version const, the `kind` enum — which is a
/// property of the schema, not of any document it claims to describe.
///
/// Paired with the regeneration gate above, this covers what the
/// generator emits as well: the committed documents are pinned equal
/// to a fresh emission, so validating them validates that emission.
#[test]
fn committed_sourcemaps_validate_against_the_wire_schema() {
    let sidecars = committed_sidecars();
    assert!(
        sidecars.len() >= MIN_SIDECARS,
        "found only {} committed sidecars; expected at least \
         {MIN_SIDECARS}. A walk that finds nothing certifies nothing.",
        sidecars.len(),
    );

    let mut violations: Vec<String> = Vec::new();
    for sidecar in &sidecars {
        let instance: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(sidecar).expect("read sidecar"))
                .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", sidecar.display()));
        let msgs = sourcemap_schema_violations(&instance);
        if !msgs.is_empty() {
            violations.push(format!("\n{}: {msgs:?}", sidecar.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "{} of {} committed sidecars violate \
         schemas/sce-sourcemap.v1.schema.json:{}",
        violations.len(),
        sidecars.len(),
        violations
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(""),
    );
}

/// A sidecar the schema must reject, built by changing exactly one
/// thing in a committed document.
///
/// The control assertion pins the rejection to the mutated field. A
/// hand-typed document rejects for whichever constraint it trips
/// first, which makes a negative case pass while proving nothing
/// about the constraint it is named after.
fn assert_one_change_is_rejected(
    why: &str,
    mutate: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) {
    let sidecars = committed_sidecars();
    let first = sidecars.first().expect("at least one committed sidecar");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(first).expect("read sidecar"))
            .expect("sidecar is JSON");
    assert!(
        sourcemap_schema_violations(&doc).is_empty(),
        "the control document must be valid before mutation, \
         otherwise the rejection below proves nothing: {}",
        first.display(),
    );
    mutate(doc.as_object_mut().expect("sidecar is an object"));
    assert!(
        !sourcemap_schema_violations(&doc).is_empty(),
        "schema must reject this document ({why}): {doc}",
    );
}

#[test]
fn sourcemap_schema_rejects_an_unknown_symbol_kind() {
    assert_one_change_is_rejected("kind outside the declared enum", |obj| {
        let symbols = obj
            .get_mut("symbols")
            .and_then(|s| s.as_object_mut())
            .expect("symbols object");
        let key = symbols.keys().next().expect("at least one symbol").clone();
        symbols[&key]["kind"] = serde_json::Value::String("no-such-kind".to_string());
    });
}

#[test]
fn sourcemap_schema_rejects_a_missing_required_field() {
    assert_one_change_is_rejected("line_range absent", |obj| {
        let symbols = obj
            .get_mut("symbols")
            .and_then(|s| s.as_object_mut())
            .expect("symbols object");
        let key = symbols.keys().next().expect("at least one symbol").clone();
        symbols[&key]
            .as_object_mut()
            .expect("symbol is an object")
            .remove("line_range");
    });
}

/// The schema closes both the document and each symbol. An emitter
/// that starts writing a field without declaring it would otherwise
/// ship sidecars every external validator rejects.
#[test]
fn sourcemap_schema_rejects_an_undeclared_field() {
    assert_one_change_is_rejected("field not declared in properties", |obj| {
        obj.insert("wcet_budget_us".to_string(), serde_json::Value::from(42u32));
    });
}
