// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end contract test for `sce-codegen check`.
//
// Two claims are load-bearing and neither is self-evident from the
// implementation:
//
//   1. **Same verdict.** For every document and every backend,
//      `check -l X` agrees with `generate -l X` on both the exit code
//      and the diagnostic code. `check` is not a validation subset that
//      happens to agree today — the agreement is the contract, and this
//      test is what keeps the two call sequences from drifting apart as
//      validators are added on one side only.
//
//   2. **Writes nothing.** `check` creates no file anywhere: not in the
//      working directory, not beside the input. Asserted by running it
//      with a scratch directory as cwd and with an input staged in a
//      second scratch directory, then re-listing both.
//
// The sweep also pins the exit-code split between the two refusal axes:
// a document-axis refusal is fatal with or without `--language`, while
// a backend-axis refusal is fatal only when the operator named the
// backend.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`; removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Every entry name currently in the directory, sorted.
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .expect("read scratch dir")
            .map(|e| {
                e.expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Backends, in the order `Language::ALL` declares them.
const BACKENDS: &[&str] = &["rust", "cpp", "kotlin", "go", "python", "c11"];

/// What one invocation reported: process exit status and, when it
/// failed, the `code` of the first NDJSON diagnostic on stderr.
#[derive(Debug, PartialEq, Eq)]
struct Verdict {
    exit: Option<i32>,
    code: Option<String>,
}

fn first_diagnostic_code(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with('{') {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        v.get("code")?.as_str().map(|s| s.to_string())
    })
}

fn run(args: &[&str], cwd: &Path) -> (Verdict, String) {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .arg("--error-format=json")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (
        Verdict {
            exit: out.status.code(),
            code: first_diagnostic_code(&stderr),
        },
        stdout,
    )
}

/// Documents spanning the axes the sweep needs to cover: a clean
/// statechart, a forge kind, a document every backend but Rust refuses,
/// and one the document axis refuses outright.
fn corpus() -> Vec<PathBuf> {
    let root = repo_root().join("sce-build/tests/fixtures");
    let mut found: Vec<PathBuf> = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("scxml") {
                found.push(path);
            }
        }
    }
    found.sort();
    // Every fixture, deliberately. An earlier revision capped the sweep
    // and the cap silently dropped the documents that exercise the
    // static-generatable gate — a `check` that skipped that validator
    // stayed green here. A parity sweep that covers only part of the
    // corpus certifies only that part.
    found
}

/// Documents the fixture tree does not contain, one per refusal axis
/// the parity sweep must actually exercise.
///
/// The fixture corpus is overwhelmingly well-formed input: sweeping all
/// 66 of them still never reaches the static-generatable gate, so a
/// `check` that skipped that validator entirely stayed green against
/// the whole tree. These are the documents that make the sweep's claim
/// real — each one is refused at a different point in the pipeline, and
/// `check` must refuse it exactly where `generate` does.
const SYNTHETIC_AXES: &[(&str, &str)] = &[
    (
        "axis_initial_unknown.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="axis_initial_unknown" initial="nosuch">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    ),
    (
        "axis_no_initial.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="axis_no_initial">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    ),
    (
        "axis_malformed_xml.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="axis_malformed" initial="a">
  <state id="a"><transition event="go" target="b"/>
</scxml>
"#,
    ),
    (
        "axis_unknown_target.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="axis_unknown_target" initial="a">
  <state id="a"><transition event="go" target="nowhere"/></state>
  <final id="b"/>
</scxml>
"#,
    ),
    (
        "axis_clean.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="axis_clean" initial="a">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    ),
];

/// Stage every synthetic axis document into `dir` and return the paths.
fn stage_synthetic_axes(dir: &Path) -> Vec<PathBuf> {
    SYNTHETIC_AXES
        .iter()
        .map(|(name, body)| {
            let path = dir.join(name);
            std::fs::write(&path, body).expect("stage synthetic axis document");
            path
        })
        .collect()
}

/// Lower bound on the (document, backend) pairs the sweep must compare.
/// A sweep that finds no fixtures would pass every per-pair assertion
/// vacuously, and a shrinking corpus must fail loudly rather than
/// quietly narrow the claim.
const MIN_COMPARED_PAIRS: usize = 390;

/// Lower bound on how many of the compared pairs must be refusals.
///
/// Parity over accepted documents alone is the weaker half of the
/// claim: two pipelines agree trivially when neither rejects anything.
/// The refusals are where a missing validator on one side shows up.
const MIN_COMPARED_REFUSALS: usize = 20;

/// `check -l X` and `generate -l X` reach the same verdict.
///
/// The one intended difference is the filesystem: `generate` writes its
/// output into the scratch directory, `check` writes nothing. Exit code
/// and diagnostic code must match exactly.
#[test]
fn check_and_generate_agree_on_every_document_and_backend() {
    let mut docs = corpus();
    assert!(
        !docs.is_empty(),
        "fixture sweep found no .scxml documents to compare",
    );
    let axes_dir = ScratchDir::new("check-parity-axes");
    docs.extend(stage_synthetic_axes(axes_dir.path()));

    let out_dir = ScratchDir::new("check-parity-out");
    let cwd = repo_root();
    let mut compared = 0usize;
    let mut refusals = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    for doc in &docs {
        let doc_str = doc.to_str().expect("fixture path is utf8");
        for backend in BACKENDS {
            let (gen_verdict, _) = run(
                &[
                    "generate",
                    doc_str,
                    "-l",
                    backend,
                    "-o",
                    out_dir.path().to_str().unwrap(),
                ],
                &cwd,
            );
            let (check_verdict, _) = run(&["check", doc_str, "-l", backend], &cwd);
            compared += 1;
            if gen_verdict.exit != Some(0) {
                refusals += 1;
            }
            if gen_verdict != check_verdict {
                disagreements.push(format!(
                    "{} [{backend}]: generate={gen_verdict:?} check={check_verdict:?}",
                    doc.file_name().unwrap().to_string_lossy(),
                ));
            }
        }
    }

    assert!(
        disagreements.is_empty(),
        "check and generate disagree on {} of {compared} pairs:\n  {}",
        disagreements.len(),
        disagreements.join("\n  "),
    );
    assert!(
        compared >= MIN_COMPARED_PAIRS,
        "compared only {compared} pairs; expected at least {MIN_COMPARED_PAIRS}",
    );
    assert!(
        refusals >= MIN_COMPARED_REFUSALS,
        "only {refusals} of {compared} compared pairs were refusals; expected at least \
         {MIN_COMPARED_REFUSALS}. Parity over accepted documents alone would let a \
         missing validator on the check side pass unnoticed.",
    );
}

/// `check` creates nothing — not in the working directory, not beside
/// the input.
///
/// Both locations are asserted because they fail differently: a stray
/// output directory shows up in cwd, while a sourcemap or a synthesised
/// child SCXML lands next to the document.
#[test]
fn check_writes_no_file_anywhere() {
    let cwd = ScratchDir::new("check-cwd");
    let staged = ScratchDir::new("check-input");

    let doc = staged.path().join("machine.scxml");
    std::fs::write(
        &doc,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="machine" initial="a">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    )
    .expect("stage fixture");

    let before_cwd = cwd.entries();
    let before_staged = staged.entries();

    let (verdict, stdout) = run(&["check", doc.to_str().unwrap()], cwd.path());
    assert_eq!(verdict.exit, Some(0), "clean document must check clean");

    assert_eq!(
        cwd.entries(),
        before_cwd,
        "check must not create anything in the working directory",
    );
    assert_eq!(
        staged.entries(),
        before_staged,
        "check must not create anything beside the input document",
    );

    // The manifest says so too: an empty `artifacts` array is the
    // subcommand's contract, not an accident of this input.
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("check emits one JSON manifest");
    assert_eq!(manifest["kind"], "check");
    assert_eq!(
        manifest["artifacts"].as_array().map(|a| a.len()),
        Some(0),
        "check manifest must carry an empty artifacts array: {stdout}",
    );
}

/// Every emitted `check` manifest validates against the checked-in
/// schema — the wire surface is only a contract if a produced record is
/// tested against it.
#[test]
fn check_manifest_validates_against_the_wire_schema() {
    let cwd = repo_root();
    let schema_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");

    let mut validated = 0usize;
    for doc in corpus() {
        let (verdict, stdout) = run(&["check", doc.to_str().unwrap()], &cwd);
        if verdict.exit != Some(0) {
            // A document-axis refusal emits no manifest by design.
            continue;
        }
        let instance: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!("manifest for {} is not JSON: {e}\n{stdout}", doc.display())
        });
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema_value)
            .expect("manifest schema compiles");
        let msgs: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        assert!(
            msgs.is_empty(),
            "check manifest for {} violates the wire schema: {msgs:?}\n{stdout}",
            doc.display(),
        );
        validated += 1;
    }
    assert!(
        validated > 0,
        "no check manifest was validated; the sweep proved nothing",
    );
}

/// The two refusal axes split on `--language`, and the sweep manifest
/// names the backend axis rather than collapsing it into an exit code.
///
/// `smart_light.scxml` is the document that separates them: it is
/// well-formed, and its `cond="cpp:…"` guard is C++ SOURCE — so only the
/// C++ backend can lower it, and that is a property of the construct
/// rather than of how much has been written yet.
///
/// It used to be `statechart_native_action.scxml`, whose `<sce:action>`
/// only Rust lowered. That stopped discriminating on 2026-08-24 when
/// every backend grew a native-action path: a test built on a coverage
/// gap retires itself the day the gap closes, while still reading as a
/// pass. The re-aim is at a discriminator that cannot close.
#[test]
fn backend_axis_refusal_is_fatal_only_when_the_backend_was_named() {
    let doc = repo_root().join("examples/smart_light/smart_light.scxml");
    assert!(doc.exists(), "fixture missing: {}", doc.display());
    let cwd = repo_root();
    let doc_str = doc.to_str().unwrap();

    // Named backend that refuses → fatal, mirroring `generate -l rust`.
    let (named, _) = run(&["check", doc_str, "-l", "rust"], &cwd);
    assert_ne!(
        named.exit,
        Some(0),
        "a named backend's refusal must be fatal",
    );
    assert_eq!(named.code.as_deref(), Some("generate/unsupported-feature"));

    // Named backend that accepts → clean.
    let (cpp, _) = run(&["check", doc_str, "-l", "cpp"], &cwd);
    assert_eq!(cpp.exit, Some(0), "C++ owns `cpp:` and must lower it");

    // No backend named → sweep, exit 0, per-backend verdicts on the wire.
    let (swept, stdout) = run(&["check", doc_str], &cwd);
    assert_eq!(
        swept.exit,
        Some(0),
        "an unnamed sweep reports backend coverage rather than failing",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("sweep emits a manifest");
    let verdicts: BTreeMap<String, String> = manifest["languages"]
        .as_array()
        .expect("sweep manifest carries languages")
        .iter()
        .map(|v| {
            (
                v["language"].as_str().expect("language").to_string(),
                v["status"].as_str().expect("status").to_string(),
            )
        })
        .collect();

    assert_eq!(
        verdicts.len(),
        BACKENDS.len(),
        "sweep must report every backend: {verdicts:?}",
    );
    assert_eq!(verdicts.get("cpp").map(String::as_str), Some("ok"));
    for backend in BACKENDS.iter().filter(|b| **b != "cpp") {
        assert_eq!(
            verdicts.get(*backend).map(String::as_str),
            Some("rejected"),
            "{backend} cannot lower a `cpp:` guard — it is C++ source — and must \
             report rejected",
        );
    }
}

/// A document-axis refusal is fatal whether or not a backend was named
/// — the document is wrong under every backend, so there is nothing for
/// a sweep to report.
#[test]
fn document_axis_refusal_is_fatal_with_or_without_a_named_backend() {
    let staged = ScratchDir::new("check-doc-axis");
    let doc = staged.path().join("broken.scxml");
    std::fs::write(
        &doc,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="broken" initial="nosuch">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    )
    .expect("stage fixture");
    let doc_str = doc.to_str().unwrap();
    let cwd = repo_root();

    for args in [
        vec!["check", doc_str],
        vec!["check", doc_str, "-l", "rust"],
        vec!["check", doc_str, "-l", "cpp"],
    ] {
        let (verdict, stdout) = run(&args, &cwd);
        assert_ne!(
            verdict.exit,
            Some(0),
            "document-axis refusal must be fatal for {args:?}",
        );
        assert_eq!(
            verdict.code.as_deref(),
            Some("validation/invalid-reference"),
            "unexpected code for {args:?}",
        );
        assert!(
            stdout.trim().is_empty(),
            "stdout must stay empty on failure ({args:?}): {stdout}",
        );
    }
}
