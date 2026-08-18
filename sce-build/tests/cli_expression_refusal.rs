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
// Five claims are pinned here, and none follows from the others:
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
//   4. **A standard method the datamodel lacks carries its repair.**
//      `words.map(...)` used to be emitted as the Lua field call
//      `words.map(...)`: `check` answered `status: "ok"` on all six
//      backends and the machine died on evaluation. The refusal names
//      the vocabulary that does exist, as `fix: replace_one_of`, so a
//      consumer repairing the document does not need Appendix B.2.
//   5. **A call on an event field is refused, and a platform field is
//      not.** W3C SCXML 5.10.1 fills seven fields of `_event` with
//      values, so `_event.name()` calls a string; the clause is a floor
//      rather than a ceiling, so `_event.raw` — which W3C test178 reads
//      and this repository generates — is left alone. One rule cannot
//      be checked without the other: closing the namespace would refuse
//      a registered conformance fixture.

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

/// A document reaching for a method ECMA-262 defines and this
/// datamodel does not implement.
///
/// Written here rather than taken from the corpus because no corpus
/// document carries one — which is the whole reason the silence lasted.
/// `.map` is the case an author meets first: it parses, it lowers to
/// legal Lua, and the only thing wrong with it is that nothing on the
/// other side answers to the name.
const REACHING_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="n" expr="words.map(function(w) { return w; }).length"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

#[test]
fn an_unimplemented_standard_method_is_reported_with_the_vocabulary_that_exists() {
    let out = ScratchDir::new("builtin-refusal");
    let document = out.path().join("reaching.scxml");
    std::fs::write(&document, REACHING_DOCUMENT).expect("write document");

    let generated = run(&[
        "generate",
        document.to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);

    assert_eq!(generated.exit, Some(0), "stderr:\n{}", generated.stderr);
    let diagnostics = records(&generated.stderr);
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one refusal record, got:\n{}",
        generated.stderr
    );
    let record = &diagnostics[0];
    assert_eq!(record["code"], "expression/unsupported-builtin");
    assert_eq!(record["stage"], "expression");
    assert_eq!(record["spec"], "W3C SCXML §B.2");
    assert_eq!(
        record["actual"], ".map()",
        "the name reached for rides `actual`: {record}"
    );
    assert_eq!(record["fix"]["kind"], "replace_one_of");
    let candidates: Vec<String> = record["fix"]["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c.as_str().expect("candidate is a string").to_string())
        .collect();
    assert!(
        candidates.contains(&".join()".to_string()) && candidates.contains(&".slice()".to_string()),
        "the repair does not carry the vocabulary that exists: {candidates:?}"
    );
    // Non-overlap (SCE_ERROR_CONTRACT.md §3.2): the candidate list has
    // one home, and for this code it is `fix`.
    assert!(
        record["expected"].is_null(),
        "expected must stay absent: {record}"
    );
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(10),
        "the record points at the `<assign>` that wrote it: {record}"
    );

    // §5.9.1 still applies: the document generates and raises at
    // evaluation, exactly as a refused `cond` does.
    assert!(
        out.entries().iter().any(|n| n.ends_with("_sm.rs")),
        "no artifact written: {:?}",
        out.entries()
    );

    let linted = run(&[
        "check",
        document.to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);
    assert_ne!(
        linted.exit,
        Some(0),
        "--lint must refuse: {}",
        linted.stderr
    );
}

/// A namespace written as the call itself, in the shape a document
/// reaching for a constructor writes it.
///
/// `new Object()` rather than `Object()` on purpose: the operator was
/// dropped before the callee was read, so the constructor form reached
/// Lua as `Object()` and the plain form reached it unchanged. One rule
/// has to answer both.
const CONSTRUCTING_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel>
    <data id="bag" expr="0"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="bag" expr="new Object()"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

#[test]
fn a_namespace_written_as_a_call_is_reported_with_the_members_that_may_stand_there() {
    let out = ScratchDir::new("namespace-refusal");
    let document = out.path().join("constructing.scxml");
    std::fs::write(&document, CONSTRUCTING_DOCUMENT).expect("write document");

    let generated = run(&[
        "generate",
        document.to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);

    assert_eq!(generated.exit, Some(0), "stderr:\n{}", generated.stderr);
    let diagnostics = records(&generated.stderr);
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one refusal record, got:\n{}",
        generated.stderr
    );
    let record = &diagnostics[0];
    assert_eq!(record["code"], "expression/namespace-not-callable");
    assert_eq!(record["stage"], "expression");
    assert_eq!(record["spec"], "W3C SCXML §B.2");
    assert_eq!(
        record["actual"], "Object()",
        "the namespace reached for rides `actual`: {record}"
    );
    let expected: Vec<String> = record["expected"]
        .as_array()
        .expect("expected carries the members")
        .iter()
        .map(|c| c.as_str().expect("member is a string").to_string())
        .collect();
    assert!(
        expected.contains(&"Object.keys".to_string()),
        "the members that may stand there are missing: {expected:?}"
    );
    // Non-overlap (SCE_ERROR_CONTRACT.md §3.2): this code's home for a
    // candidate set is `expected`, so no `fix` may ride. The claim is
    // not decoration — dropping the call is what a `fix` would say, and
    // `Object` alone is refused too.
    assert!(record["fix"].is_null(), "fix must stay absent: {record}");
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(9),
        "the record points at the `<assign>` that wrote it: {record}"
    );

    // §5.9.1 still applies: refused expressions generate and raise at
    // evaluation rather than failing the build.
    assert!(
        out.entries().iter().any(|n| n.ends_with("_sm.rs")),
        "no artifact written: {:?}",
        out.entries()
    );

    let linted = run(&[
        "check",
        document.to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);
    assert_ne!(
        linted.exit,
        Some(0),
        "--lint must refuse: {}",
        linted.stderr
    );
}

/// The namespace read as a value, in the position where the six
/// backends stopped agreeing about what the document means.
const READING_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel>
    <data id="held" expr="0"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="held" expr="Math"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

#[test]
fn a_namespace_read_as_a_value_is_reported_with_both_halves_of_its_vocabulary() {
    let out = ScratchDir::new("namespace-read");
    let document = out.path().join("reading.scxml");
    std::fs::write(&document, READING_DOCUMENT).expect("write document");

    let generated = run(&[
        "generate",
        document.to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);

    assert_eq!(generated.exit, Some(0), "stderr:\n{}", generated.stderr);
    let diagnostics = records(&generated.stderr);
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one refusal record, got:\n{}",
        generated.stderr
    );
    let record = &diagnostics[0];
    assert_eq!(record["code"], "expression/namespace-not-a-value");
    assert_eq!(
        record["actual"], "Math",
        "the bare name is what the consumer edits: {record}"
    );
    let expected: Vec<String> = record["expected"]
        .as_array()
        .expect("expected carries the members")
        .iter()
        .map(|c| c.as_str().expect("member is a string").to_string())
        .collect();
    assert!(
        expected.contains(&"Math.PI".to_string()),
        "a read may name a constant, so the list carries both halves: {expected:?}"
    );
    assert!(record["fix"].is_null(), "fix must stay absent: {record}");

    let linted = run(&[
        "check",
        document.to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);
    assert_ne!(
        linted.exit,
        Some(0),
        "--lint must refuse: {}",
        linted.stderr
    );
}

/// A literal called, in the shape a mis-typed expression reaches — and
/// the one whose lowering was not merely wrong but unparseable.
const CALLING_A_LITERAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <datamodel>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="n" expr="1()"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

#[test]
fn a_literal_written_as_a_call_is_reported_with_neither_a_choice_nor_a_fix() {
    let out = ScratchDir::new("literal-call");
    let document = out.path().join("calling.scxml");
    std::fs::write(&document, CALLING_A_LITERAL).expect("write document");

    let generated = run(&[
        "generate",
        document.to_str().unwrap(),
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);

    assert_eq!(generated.exit, Some(0), "stderr:\n{}", generated.stderr);
    let diagnostics = records(&generated.stderr);
    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one refusal record, got:\n{}",
        generated.stderr
    );
    let record = &diagnostics[0];
    assert_eq!(record["code"], "expression/literal-not-callable");
    assert_eq!(record["actual"], "the number literal");
    // Both fields absent is this record's whole shape: the producer
    // knows what was written and nothing about what belongs instead.
    assert!(record["fix"].is_null(), "fix must stay absent: {record}");
    assert!(
        record["expected"].is_null(),
        "expected must stay absent: {record}"
    );

    let linted = run(&[
        "check",
        document.to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);
    assert_ne!(
        linted.exit,
        Some(0),
        "--lint must refuse: {}",
        linted.stderr
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

/// A guard that calls a field of `_event`.
///
/// W3C SCXML 5.10.1 fills `name` with a character string, so this guard
/// calls a string. It used to generate on all six backends with `check
/// --lint` answering exit 0 and no record on any stream; the machine
/// died evaluating the guard.
const EVENT_FIELD_CALL_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0">
  <state id="s0">
    <transition event="go" cond="_event.name() == 'go'" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The refusal reaches every backend, and carries the repair.
///
/// Every backend, because they do not all lower author ECMAScript the
/// same way — C++ and Kotlin run the author's expression on a JavaScript
/// engine and borrow only the acceptance verdict — and a rule that
/// reached five of six would leave the sixth generating a machine that
/// dies where the others are answered at design time.
#[test]
fn a_call_on_an_event_field_is_refused_by_every_backend() {
    let out = ScratchDir::new("event-field-call");
    let document = out.path().join("event_field_call.scxml");
    std::fs::write(&document, EVENT_FIELD_CALL_DOCUMENT).expect("write document");

    for language in ["rust", "cpp", "kotlin", "go", "python", "c11"] {
        let generated = ScratchDir::new(&format!("event-field-call-{language}"));
        let run = run(&[
            "generate",
            document.to_str().unwrap(),
            "-l",
            language,
            "-o",
            generated.path().to_str().unwrap(),
            "--error-format=json",
        ]);
        let diagnostics = records(&run.stderr);
        assert_eq!(
            diagnostics.len(),
            1,
            "{language} reported {} record(s):\n{}",
            diagnostics.len(),
            run.stderr
        );
        let record = &diagnostics[0];
        assert_eq!(
            record["code"], "expression/property-not-callable",
            "{language} refused on the wrong axis: {record}"
        );
        assert_eq!(record["actual"], "_event.name()");
        // ECMA-262 11.2.3 leaves one repair, and the call carries no
        // arguments to strand, so it rides `replace_with`.
        assert_eq!(record["fix"]["kind"], "replace_with");
        assert_eq!(record["fix"]["to"], "_event.name");
        assert_eq!(
            record["location"]["line"].as_u64(),
            Some(5),
            "{language} pointed at the wrong line: {record}"
        );
        // §5.9.1: a cond the processor cannot evaluate still generates.
        assert_eq!(run.exit, Some(0), "{language} stderr:\n{}", run.stderr);
    }

    let linted = run(&[
        "check",
        document.to_str().unwrap(),
        "--lint",
        "--error-format=json",
    ]);
    assert_ne!(
        linted.exit,
        Some(0),
        "--lint must refuse: {}",
        linted.stderr
    );
}

/// An event field the specification does not name is not refused.
///
/// W3C test178 reads `_event.raw`, which 5.10.1 never mentions and an
/// Event I/O Processor supplies. It is a registered conformance fixture,
/// so the rule above has to be stated about the fields the clause names
/// rather than about `_event` as a closed namespace — the shape `Math`
/// has. Read from the fixture on disk rather than from a copy of the
/// expression: a rule that closed the namespace would still pass against
/// a copy nobody generates.
#[test]
fn a_platform_field_of_the_event_is_not_refused() {
    const DOCUMENT: &str = "resources/178/test178.scxml";
    let source = std::fs::read_to_string(repo_root().join(DOCUMENT)).expect("read test178");
    assert!(
        source.contains("_event.raw"),
        "{DOCUMENT} no longer carries the expression this test exists for"
    );

    let out = ScratchDir::new("event-platform-field");
    let run = run(&[
        "generate",
        DOCUMENT,
        "-l",
        "rust",
        "-o",
        out.path().to_str().unwrap(),
        "--error-format=json",
    ]);
    assert_eq!(run.exit, Some(0), "stderr:\n{}", run.stderr);
    let expression_records: Vec<_> = records(&run.stderr)
        .into_iter()
        .filter(|r| r["stage"] == "expression")
        .collect();
    assert!(
        expression_records.is_empty(),
        "a registered conformance fixture was refused: {expression_records:?}"
    );
    assert!(
        out.entries().iter().any(|n| n.ends_with("_sm.rs")),
        "no artifact written: {:?}",
        out.entries()
    );
}
