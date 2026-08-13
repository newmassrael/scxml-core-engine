// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// How a caller *names* an input does not change what the tool says
// about it.
//
// A document has three spellings from the directory it lives in —
// `door.scxml`, `./door.scxml`, and the absolute path — and they
// address one file. Nothing in the pipeline is allowed to depend on
// which one arrived, so every subcommand must return the same exit
// status, the same diagnostic code, and (where it emits) the same
// bytes for all three.
//
// It did not. `Path::parent` answers a lexical question: for a path
// carrying no separator it returns `Some("")`, not `None`, and `""`
// names no directory that `read_dir` can open. Four sites inferred a
// directory that way and every one of them carried an
// `unwrap_or(".")` written for exactly this case — none fired,
// because the case is `Some("")`. What a caller saw:
//
//   * `generate` and `orchestrate` refused a clean document with
//     `cli/read-input` at exit 20 and an **empty path** in the
//     message ("Cannot read : ... failed to read :"), because the
//     path that failed to read was the empty string. The same
//     document by absolute path generated at exit 0.
//   * `generate-conformance` refused `--manifest fixtures.json` with
//     "cannot derive resource_dir from manifest path", and silently
//     resolved `--manifest ./fixtures.json` to a *different*
//     directory — `resources/` under the working directory instead of
//     beside it — then failed further downstream with a code that
//     named neither.
//
// The shortest way to name a document is the one a person types:
// `cd resources && sce-codegen generate door.scxml`. It was the one
// spelling that did not work.
//
// Why no existing gate saw it: `cli_check`'s parity sweep compares
// `check -l X` against `generate -l X` over the whole fixture corpus,
// which is the right claim on a different axis — it holds every path
// fixed and varies the subcommand. Every path it passes is absolute,
// built from `repo_root()`. The defect lives in the axis that sweep
// holds constant, so no amount of widening its corpus could reach it.
// This file varies the axis instead.

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
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Lower bound on invocations executed by the sweep.
///
/// The comparison is per naming-group, so a sweep that lost its
/// documents or its subcommand list would compare nothing and pass
/// every assertion vacuously.
const MIN_INVOCATIONS: usize = 30;

/// Lower bound on naming groups whose shared verdict is a *refusal*.
///
/// Parity over accepted inputs is the weaker half: three namings of a
/// document nothing rejects agree trivially. The refusals are where a
/// path inferred from the wrong spelling changes which validator
/// speaks — and the original defect turned one refusal into an
/// entirely different one (`import/file-not-found` into
/// `cli/read-input`).
const MIN_REFUSING_GROUPS: usize = 3;

/// What one invocation reported.
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

/// Run the binary in `cwd` and report the verdict.
///
/// `SOURCE_DATE_EPOCH` is pinned so the `generated-at` line in any
/// emitted file is the same for every naming; without it the byte
/// comparison below would fail on the clock rather than on the path.
fn run(args: &[String], cwd: &Path) -> Verdict {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .arg("--error-format=json")
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    Verdict {
        exit: out.status.code(),
        code: first_diagnostic_code(&String::from_utf8_lossy(&out.stderr)),
    }
}

/// One emitted file: its path relative to the output directory, and
/// its bytes.
///
/// Bytes rather than a digest so a mismatch names the file and the
/// reader can see which line moved.
type EmittedFile = (String, Vec<u8>);

/// Every file under `dir`, sorted by path.
fn tree_contents(dir: &Path) -> Vec<EmittedFile> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                let rel = path
                    .strip_prefix(dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, bytes));
            }
        }
    }
    out.sort();
    out
}

/// The three ways to name `file_name` from the directory holding it.
///
/// `.` and the bare name are the two that differ lexically only in a
/// prefix a person would never think mattered, which is why both are
/// here: the original defect refused one and accepted the other.
///
/// Every comparison below is *between* the members of this list, so a
/// list that lost a member would compare fewer spellings and report
/// the same green. The count is asserted here rather than at the call
/// sites so one check covers all three sweeps.
fn namings(dir: &Path, file_name: &str) -> Vec<(&'static str, String)> {
    let all = vec![
        ("bare", file_name.to_string()),
        ("dot-relative", format!("./{file_name}")),
        (
            "absolute",
            dir.join(file_name).to_string_lossy().into_owned(),
        ),
    ];
    assert_eq!(
        all.len(),
        3,
        "every spelling of a path is a member of the comparison; dropping one \
         narrows the claim silently"
    );
    let distinct: std::collections::BTreeSet<&str> = all.iter().map(|(_, s)| s.as_str()).collect();
    assert_eq!(
        distinct.len(),
        all.len(),
        "two namings resolved to the same string, so the pair compares nothing: {all:?}"
    );
    all
}

/// A clean statechart — the accepted half of the claim.
const CLEAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="door" initial="closed">
  <state id="closed"><transition event="open" target="opened"/></state>
  <final id="opened"/>
</scxml>
"#;

/// A forge document whose `<sce:import src>` resolves to nothing.
///
/// The refusal is raised in the `import` stage, past the point where
/// the inferred source-set root is consulted, so a root inferred from
/// the wrong spelling replaced this document's own diagnostic with a
/// CLI-boundary one. That substitution is what the refusal half of
/// this sweep pins.
const BROKEN_IMPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="importer" version="1.0">
  <sce:import as="peer" src="no_such_sibling.scxml" kind="codec"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="peer"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>
"#;

/// A document named three ways is one document.
#[test]
fn naming_a_document_does_not_change_the_verdict() {
    let docs = ScratchDir::new("naming-docs");
    std::fs::write(docs.path().join("door.scxml"), CLEAN).expect("stage clean document");
    std::fs::write(docs.path().join("importer.scxml"), BROKEN_IMPORT)
        .expect("stage forge document");

    // `-o` is a fixed absolute path in every run so the only thing
    // varying between the members of a group is how the *input* is
    // named.
    let out = ScratchDir::new("naming-out");
    let out_arg = out.path().to_string_lossy().into_owned();

    let mut invocations = 0usize;
    let mut refusing_groups = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    for doc in ["door.scxml", "importer.scxml"] {
        for backend in ["rust", "cpp", "go"] {
            for template in [
                vec!["generate", "{doc}", "-l", backend, "-o", &out_arg],
                vec!["check", "{doc}", "-l", backend],
                vec![
                    "orchestrate",
                    "--scxml",
                    "{doc}",
                    "-l",
                    backend,
                    "-o",
                    &out_arg,
                ],
            ] {
                let mut group: Vec<(&str, Verdict)> = Vec::new();
                for (label, spelling) in namings(docs.path(), doc) {
                    let args: Vec<String> = template
                        .iter()
                        .map(|a| {
                            if *a == "{doc}" {
                                spelling.clone()
                            } else {
                                (*a).to_string()
                            }
                        })
                        .collect();
                    group.push((label, run(&args, docs.path())));
                    invocations += 1;
                }

                let (_, first) = &group[0];
                if first.exit != Some(0) {
                    refusing_groups += 1;
                }
                for (label, verdict) in &group[1..] {
                    if verdict != first {
                        disagreements.push(format!(
                            "{} {template:?}: {} => {first:?} but {label} => {verdict:?}",
                            doc, group[0].0,
                        ));
                    }
                }
            }
        }
    }

    assert!(
        disagreements.is_empty(),
        "naming the input changed the verdict in {} of {invocations} invocations:\n  {}",
        disagreements.len(),
        disagreements.join("\n  "),
    );
    assert!(
        invocations >= MIN_INVOCATIONS,
        "ran only {invocations} invocations; expected at least {MIN_INVOCATIONS}",
    );
    assert!(
        refusing_groups >= MIN_REFUSING_GROUPS,
        "only {refusing_groups} naming groups refused; expected at least \
         {MIN_REFUSING_GROUPS}. Parity over accepted inputs alone would let a \
         path inferred from the wrong spelling swap one refusal for another \
         unnoticed.",
    );
}

/// Beyond the verdict: the emitted files are byte-identical.
///
/// Separate from the verdict sweep because it is a strictly stronger
/// claim and fails differently — a run can exit 0 three times and
/// still embed a `source-hash` computed over three different input
/// sets, which is precisely what an inferred root gets wrong. The
/// digest is a fold over paths *relative* to that root, so it survives
/// the spelling; this asserts that rather than assuming it.
///
/// `--source-root` is pinned because provenance is documented to echo
/// the path as the caller named it ("the path is emitted exactly as
/// named on the command line"), so the `// From:` line is *expected*
/// to differ between spellings. Pinning the root re-expresses all
/// three against one directory, which is what makes the rest of the
/// file comparable.
#[test]
fn naming_a_document_does_not_change_the_generated_bytes() {
    let docs = ScratchDir::new("naming-bytes-docs");
    std::fs::write(docs.path().join("door.scxml"), CLEAN).expect("stage clean document");
    let source_root = docs.path().to_string_lossy().into_owned();

    let mut baseline: Option<(String, Vec<EmittedFile>)> = None;
    for (label, spelling) in namings(docs.path(), "door.scxml") {
        let out = ScratchDir::new(&format!("naming-bytes-{label}"));
        let verdict = run(
            &[
                "generate".to_string(),
                spelling,
                "-l".to_string(),
                "rust".to_string(),
                "-o".to_string(),
                out.path().to_string_lossy().into_owned(),
                "--source-root".to_string(),
                source_root.clone(),
            ],
            docs.path(),
        );
        assert_eq!(
            verdict.exit,
            Some(0),
            "{label} naming must generate: {verdict:?}"
        );

        let contents = tree_contents(out.path());
        assert!(
            !contents.is_empty(),
            "{label} naming produced no files to compare"
        );
        match &baseline {
            None => baseline = Some((label.to_string(), contents)),
            Some((base_label, base)) => {
                assert_eq!(
                    base.len(),
                    contents.len(),
                    "{base_label} emitted {} files, {label} emitted {}",
                    base.len(),
                    contents.len(),
                );
                for ((base_name, base_bytes), (name, bytes)) in base.iter().zip(contents.iter()) {
                    assert_eq!(
                        base_name, name,
                        "{base_label} and {label} emitted \
                                different file names"
                    );
                    assert_eq!(
                        String::from_utf8_lossy(base_bytes),
                        String::from_utf8_lossy(bytes),
                        "{base_name}: {base_label} and {label} namings emitted different bytes",
                    );
                }
            }
        }
    }
}

/// The manifest-taking subcommands resolve the same `resources/`
/// whichever way the manifest is named.
///
/// Held apart from the document sweep because the derivation is one
/// step longer — these want the directory *beside* the one holding
/// the manifest — and that extra step is where the lexical answer
/// runs out: the parent of `.` has no spelling without `..`. The bare
/// naming refused outright and the `./` naming resolved to a
/// different directory, so all three outcomes were distinct.
#[test]
fn naming_a_manifest_does_not_change_the_resolved_resources() {
    let manifest_dir = repo_root().join("tests/forge/conformance");
    assert!(
        manifest_dir.join("fixtures.json").is_file(),
        "conformance manifest fixture is missing"
    );

    let mut verdicts: Vec<(String, Verdict)> = Vec::new();
    let mut listings: Vec<(String, Vec<u8>)> = Vec::new();

    for (label, spelling) in namings(&manifest_dir, "fixtures.json") {
        let out = ScratchDir::new(&format!("naming-manifest-{label}"));
        verdicts.push((
            label.to_string(),
            run(
                &[
                    "generate-conformance".to_string(),
                    "--manifest".to_string(),
                    spelling.clone(),
                    "-o".to_string(),
                    out.path().to_string_lossy().into_owned(),
                    "-l".to_string(),
                    "rust".to_string(),
                ],
                &manifest_dir,
            ),
        ));

        // `list-fixtures` derives the same `resources/` for its
        // per-fixture backend filter, so a root resolved from the
        // wrong spelling shows up as a *different list* rather than as
        // a failure — a silence the exit status cannot report.
        //
        // The language matters. The filter reads each fixture's SCXML
        // out of the resolved directory to decide whether it is
        // MCU-only, and excludes those from backends that cannot carry
        // them; a directory that resolves to nothing makes every file
        // unreadable, the check unanswerable, and nothing excluded.
        // `rust` and `c11` are the two backends that exclude nothing
        // even when the directory is right, so listing under either of
        // them is identical whether or not the resolution worked —
        // which is exactly the probe that cannot reach the defect. The
        // control below pins that this one can.
        listings.push((
            label.to_string(),
            list_fixtures(&manifest_dir, &spelling, "cpp"),
        ));
    }

    let (base_label, base_verdict) = &verdicts[0];
    for (label, verdict) in &verdicts[1..] {
        assert_eq!(
            verdict, base_verdict,
            "generate-conformance: {base_label} => {base_verdict:?} but {label} => {verdict:?}",
        );
    }
    assert_eq!(
        base_verdict.exit,
        Some(0),
        "the control naming must succeed, or the comparison above is between \
         three identical failures: {base_verdict:?}",
    );

    // Control: the filter must actually exclude something under the
    // correct resolution, or all three listings agree for a reason
    // that has nothing to do with the path.
    let unfiltered = list_fixtures(&manifest_dir, "fixtures.json", "c11");
    let filtered = &listings[0].1;
    assert!(
        !filtered.is_empty() && filtered.len() < unfiltered.len(),
        "the backend filter excluded nothing ({} of {} lines), so a resource \
         directory resolved to the wrong place would list exactly the same \
         fixtures as one resolved correctly",
        filtered.len(),
        unfiltered.len(),
    );

    let (base_label, base_listing) = &listings[0];
    for (label, listing) in &listings[1..] {
        assert_eq!(
            String::from_utf8_lossy(base_listing),
            String::from_utf8_lossy(listing),
            "list-fixtures: {base_label} and {label} namings resolved different \
             resource directories",
        );
    }
}

/// `list-fixtures` output for one manifest spelling and backend.
fn list_fixtures(cwd: &Path, manifest: &str, language: &str) -> Vec<u8> {
    let out = Command::new(sce_codegen_bin())
        .args([
            "list-fixtures",
            "--manifest",
            manifest,
            "--catalog",
            "forge",
            "--language",
            language,
        ])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        out.status.success(),
        "list-fixtures failed for {manifest:?} [{language}]: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    out.stdout
}
