// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The contract for a refusal that does not fail the build.
//
// W3C SCXML §5.9.1 obliges a `cond` the processor cannot evaluate to
// raise `error.execution` and read as false — tests 309 and 344 write
// `cond="return"` for exactly that — so SCE must generate the document
// rather than refuse it. What SCE must *not* do is reach that verdict
// silently, which is what it did: `check` answered `status: "ok"`,
// `generate` exited `0` listing artifacts, and the parser's message
// survived only inside a string literal in the generated source, where
// its one reader was whoever later ran the machine.
//
// Three claims are pinned here, and none follows from the others:
//
//   1. **Reported, and still generated.** The diagnostic is on stderr,
//      the manifest is on stdout, the exit is `0`, and the artifact is
//      written. `SCE_ERROR_CONTRACT.md` §1 and §10.2 already give a
//      consumer everything it needs to tell this apart from a fatal
//      refusal, so no severity field is invented for it.
//   2. **`--lint` makes it fatal.** The flag already separates the two
//      kinds of author: the design-time lints are off by default
//      because the W3C corpus declares unreachable states on purpose,
//      and `scripts/gates/example-codegen.sh` turns them on for every
//      document this repository writes. A refused expression is the
//      same shape of claim — conformance obliges SCE to generate one,
//      nothing obliges an author to write one.
//   3. **A native `cond` is a backend-axis rejection.** `cpp:` and
//      `kt:` name the language the guard is written in, and only that
//      backend can lower it. Every other one used to emit either
//      uncompilable source or a guard that always raises.

use std::path::{Path, PathBuf};
use std::process::Command;
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

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

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

struct Run {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Run {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    Run {
        exit: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Every NDJSON record on stderr.
fn records(stderr: &str) -> Vec<serde_json::Value> {
    stderr
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).expect("stderr line is one JSON object"))
        .collect()
}

/// W3C test 344 writes `cond="return"` on purpose, so the refusal under
/// test is one the conformance suite requires SCE to keep generating.
const REFUSING_DOCUMENT: &str = "resources/344/test344.scxml";

#[test]
fn a_refused_expression_is_reported_and_the_document_still_generates() {
    let out = ScratchDir::new("refusal-generate");
    let run = run(&[
        "generate",
        REFUSING_DOCUMENT,
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);

    assert_eq!(run.exit, Some(0), "stderr:\n{}", run.stderr);

    let diagnostics = records(&run.stderr);
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one refusal record, got:\n{}",
        run.stderr
    );
    let record = &diagnostics[0];
    assert_eq!(record["code"], "expression/unsupported-construct");
    assert_eq!(record["stage"], "expression");
    assert_eq!(
        record["actual"], "reserved word 'return' used as a value",
        "the construct rides `actual`, not just the prose"
    );
    // The author has to be able to open the line. A location naming
    // only the file would leave them the same search the raise did.
    assert_eq!(record["location"]["file"], "test344.scxml");
    assert!(
        record["location"]["line"].as_u64().unwrap_or(0) > 0,
        "record carries no line: {record}"
    );
    assert!(
        record["message"]
            .as_str()
            .unwrap_or_default()
            .starts_with("<transition cond>"),
        "the message names which of the line's expressions was refused: {record}"
    );

    // Still a successful run: manifest on stdout, artifact on disk.
    let manifest: serde_json::Value =
        serde_json::from_str(run.stdout.trim()).expect("stdout is one manifest line");
    assert_eq!(manifest["kind"], "generate");
    assert!(
        !manifest["artifacts"]
            .as_array()
            .expect("artifacts")
            .is_empty(),
        "conformance requires this document to generate: {manifest}"
    );
    assert!(
        out.entries().iter().any(|n| n.ends_with("_sm.rs")),
        "no artifact written: {:?}",
        out.entries()
    );
}

#[test]
fn check_reports_the_same_refusal_generate_does() {
    let generate_out = ScratchDir::new("refusal-agree");
    let generated = run(&[
        "generate",
        REFUSING_DOCUMENT,
        "-l",
        "rust",
        "-o",
        generate_out.path().to_str().unwrap(),
        "--error-format=json",
    ]);
    let checked = run(&[
        "check",
        REFUSING_DOCUMENT,
        "--language",
        "rust",
        "--error-format=json",
    ]);

    assert_eq!(checked.exit, generated.exit);
    assert_eq!(
        records(&checked.stderr)
            .iter()
            .map(|r| r["id"].clone())
            .collect::<Vec<_>>(),
        records(&generated.stderr)
            .iter()
            .map(|r| r["id"].clone())
            .collect::<Vec<_>>(),
        "`check` is contracted to reach the verdict `generate` does"
    );
}

#[test]
fn lint_makes_a_refused_expression_fatal_and_writes_nothing() {
    let out = ScratchDir::new("refusal-lint");
    let run = run(&[
        "generate",
        REFUSING_DOCUMENT,
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);

    assert_ne!(run.exit, Some(0), "--lint must refuse: {}", run.stderr);
    assert_eq!(
        records(&run.stderr)[0]["code"],
        "expression/unsupported-construct"
    );
    // §10.2: on failure stdout is empty and nothing is materialised.
    assert!(run.stdout.trim().is_empty(), "stdout: {}", run.stdout);
    assert!(
        out.entries().is_empty(),
        "a refused run wrote {:?}",
        out.entries()
    );
}

/// The corpus this repository authors is lint-clean, which is what
/// `scripts/gates/example-codegen.sh` now enforces for refusals too.
///
/// Asserted here rather than left to the shell gate because the gate
/// has no CI counterpart: without this, a document that starts carrying
/// a refused expression is caught only by whoever runs the gate locally.
#[test]
fn every_authored_document_is_free_of_refused_expressions() {
    let root = repo_root();
    let listed = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "examples/*.scxml",
            "integration_resources/*/*.scxml",
        ])
        .current_dir(&root)
        .output()
        .expect("git ls-files");
    assert!(listed.status.success());

    let documents: Vec<String> = listed
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    // The corpus was 35 documents when this bound was set; a discovery
    // bug that swept nothing would otherwise read as a pass.
    assert!(
        documents.len() >= 30,
        "swept only {} authored document(s)",
        documents.len()
    );

    let mut carrying = Vec::new();
    for document in &documents {
        let run = run(&["check", document, "--error-format=json"]);
        for record in records(&run.stderr) {
            if record["stage"] == "expression" {
                carrying.push(format!("{document}: {}", record["message"]));
            }
        }
    }
    assert!(
        carrying.is_empty(),
        "authored document(s) carry a refused expression:\n{}",
        carrying.join("\n")
    );
}

/// A native `cond` names its language; only that backend lowers it.
///
/// `examples/smart_light/smart_light.scxml` writes
/// `cond="cpp:hardware.hasPower()"`. Before this refusal existed, Rust,
/// Go and C11 emitted the guard verbatim — `if cpp:hardware.hasPower()`,
/// which no compiler accepts — and Python lowered it through the
/// ECMAScript frontend, producing a guard that raises on every
/// evaluation. All four reported success and listed artifacts.
#[test]
fn a_native_cond_is_refused_by_every_backend_but_its_own() {
    const DOCUMENT: &str = "examples/smart_light/smart_light.scxml";

    let cpp = ScratchDir::new("native-cond-cpp");
    let accepted = run(&[
        "generate",
        DOCUMENT,
        "-l",
        "cpp",
        "-o",
        cpp.path().to_str().unwrap(),
        "--error-format=json",
    ]);
    assert_eq!(
        accepted.exit,
        Some(0),
        "C++ owns `cpp:` and must still lower it: {}",
        accepted.stderr
    );

    for language in ["rust", "go", "python", "c11", "kotlin"] {
        let out = ScratchDir::new(&format!("native-cond-{language}"));
        let run = run(&[
            "generate",
            DOCUMENT,
            "-l",
            language,
            "-o",
            out.path().to_str().unwrap(),
            "--error-format=json",
        ]);
        assert_ne!(
            run.exit,
            Some(0),
            "{language} accepted a native C++ guard it cannot lower"
        );
        assert_eq!(
            records(&run.stderr)[0]["code"],
            "generate/unsupported-feature",
            "{language} refused on the wrong axis"
        );
        assert!(
            out.entries().is_empty(),
            "{language} wrote {:?} for a document it refused",
            out.entries()
        );
    }
}
