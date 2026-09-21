//! `sce:unresolved` reaches the forge pipeline, not statecharts alone.
//!
//! ⚠ WHAT THIS HOLDS IN PLACE. Measured 2026-09-18, before the fix these
//! tests cover: a `sce:kind="transform"` document carrying
//! `sce:unresolved` on a `<data>` node generated under
//! `--strict-unresolved` with **exit 0**, and `sce-codegen unresolved`
//! answered the same file with "transform kind cannot be processed by
//! the SCXML pipeline" and exit 3. The marker was accepted by the XML
//! and understood by nothing.
//!
//! That is the worst shape a safety mechanism can take. An author who
//! writes down "I do not know this value" is told nothing, and the build
//! ships the guess — the mechanism's presence is what makes it
//! dangerous, because a reader of the document believes the question was
//! asked.
//!
//! ⚠⚠ THE POSITIVE CONTROL IS NOT OPTIONAL HERE. A test that only
//! checked the refusal would pass against a build that refuses every
//! forge document, so `a_forge_document_without_markers_still_builds`
//! is what separates "the check works" from "the check is stuck on".

use std::process::Command;

fn sce_codegen_bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!(
            "sce_forge_unresolved_{label}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create temp dir");
        Tmp(d)
    }
    fn write(&self, name: &str, body: &str) -> std::path::PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, body).expect("write document");
        p
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const MARKED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="marked">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"
          sce:unresolved="scale-factor"
          sce:unresolved-reason="the source states no multiplier"
          sce:unresolved-candidates="2 4 8"/>
  </datamodel>
</scxml>
"#;

/// The same document with the OTHER marker kind. Nothing else differs —
/// that is what makes the pair a discriminator rather than two examples.
const ASSUMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="assumed">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"
          sce:assumed="scale-factor"
          sce:assumed-reason="the source states no multiplier; 2 chosen pending an answer"
          sce:assumed-candidates="2 4 8"/>
  </datamodel>
</scxml>
"#;

const CLEAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="clean">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"/>
  </datamodel>
</scxml>
"#;

fn generate(doc: &std::path::Path, out: &std::path::Path, strict: bool) -> (Option<i32>, String) {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args([
        "generate",
        doc.to_str().unwrap(),
        "-l",
        "cpp",
        "-o",
        out.to_str().unwrap(),
    ]);
    if strict {
        cmd.arg("--strict-unresolved");
    }
    let run = cmd.output().expect("spawn sce-codegen");
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

#[test]
fn strict_unresolved_refuses_a_marked_forge_document() {
    let t = Tmp::new("strict");
    let doc = t.write("marked.scxml", MARKED);
    let (code, stderr) = generate(&doc, &t.0, true);
    assert_ne!(
        code,
        Some(0),
        "a forge document carrying sce:unresolved generated under --strict-unresolved"
    );
    // The refusal must name the node and the marker, because the author's
    // next action is to go answer that specific question.
    assert!(
        stderr.contains("scaled"),
        "the refusal does not name the field:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-factor"),
        "the refusal does not name the marker id:\n{stderr}"
    );
    assert!(
        stderr.contains("no multiplier"),
        "the refusal drops the author's reason:\n{stderr}"
    );
}

#[test]
fn a_forge_document_without_markers_still_builds() {
    let t = Tmp::new("clean");
    let doc = t.write("clean.scxml", CLEAN);
    let (code, stderr) = generate(&doc, &t.0, true);
    assert_eq!(
        code,
        Some(0),
        "--strict-unresolved refused a forge document with no markers:\n{stderr}"
    );
}

#[test]
fn without_the_flag_a_marked_forge_document_still_builds() {
    // The marker is metadata by default on the statechart side and must
    // stay metadata here: the flag is what lifts it, not the attribute.
    let t = Tmp::new("lenient");
    let doc = t.write("marked.scxml", MARKED);
    let (code, stderr) = generate(&doc, &t.0, false);
    assert_eq!(
        code,
        Some(0),
        "the marker failed a build that did not ask for strictness:\n{stderr}"
    );
}

#[test]
fn the_unresolved_report_reads_a_forge_document() {
    let t = Tmp::new("report");
    let doc = t.write("marked.scxml", MARKED);
    let run = Command::new(sce_codegen_bin())
        .args(["unresolved", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "`unresolved` refused a forge document:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected one record, got:\n{out}");
    for needle in [
        "\"id\":\"scale-factor\"",
        "\"node_type\":\"forge\"",
        "no multiplier",
        "\"candidates\"",
    ] {
        assert!(
            lines[0].contains(needle),
            "record is missing {needle}:\n{}",
            lines[0]
        );
    }
}

/// ⚠ THE WHOLE POINT OF THE SECOND KIND, IN ONE ASSERTION. A marker that
/// blocked would make `sce:assumed` a synonym for `sce:unresolved`, and
/// a marker that was silent would make it a synonym for deleting the
/// marker. It has to be neither, so both halves are asserted — here and
/// in the test below — against documents that differ in nothing else.
///
/// Why the distinction was needed: a real conversion hit a spec whose
/// operating logic covers two ranges and says nothing about the three
/// values between them. `unresolved` is the honest first answer, but it
/// stops the build, and a build that cannot run measures nothing else in
/// the document. Deleting the marker instead would make the document
/// claim the spec had settled a question it never mentions.
#[test]
fn an_assumed_marker_does_not_block_a_strict_build() {
    let t = Tmp::new("assumed_strict");
    let doc = t.write("assumed.scxml", ASSUMED);
    let (code, stderr) = generate(&doc, &t.0, true);
    assert_eq!(
        code,
        Some(0),
        "--strict-unresolved refused a document whose only marker is an assumption:\n{stderr}"
    );
}

#[test]
fn an_assumed_marker_is_still_reported_and_says_which_kind_it_is() {
    let t = Tmp::new("assumed_report");
    let doc = t.write("assumed.scxml", ASSUMED);
    let run = Command::new(sce_codegen_bin())
        .args(["unresolved", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(run.status.code(), Some(0));
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "an assumption vanished from the report — it is on the record, not resolved:\n{out}"
    );
    // The kind must be ON THE WIRE. A consumer rendering a work queue
    // has to separate "a question for the spec's author" from "a
    // decision already taken that the author should confirm", and
    // without this field both arrive as one undifferentiated list.
    assert!(
        lines[0].contains("\"kind\":\"assumed\""),
        "the record does not say which kind of marker it is:\n{}",
        lines[0]
    );
}

#[test]
fn an_unresolved_record_says_so_too() {
    // The other half of the pair: if `kind` were emitted only for the
    // new value, a consumer reading an older record could not tell an
    // absent field from a question.
    let t = Tmp::new("kind_unresolved");
    let doc = t.write("marked.scxml", MARKED);
    let run = Command::new(sce_codegen_bin())
        .args(["unresolved", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        out.contains("\"kind\":\"unresolved\""),
        "the record does not state its kind:\n{out}"
    );
}

#[test]
fn the_unresolved_report_is_empty_for_a_clean_forge_document() {
    // Same reason as the positive control above: a reporter that printed
    // a record for every document would satisfy the test above and tell a
    // consumer nothing.
    let t = Tmp::new("report_clean");
    let doc = t.write("clean.scxml", CLEAN);
    let run = Command::new(sce_codegen_bin())
        .args(["unresolved", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(run.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&run.stdout).trim().is_empty(),
        "a clean forge document produced a record"
    );
}

// ── A marked field with no expression ───────────────────────────
//
// Every document above carries an `expr` *as well as* a marker, which is
// the case where the author has something to generate and is flagging
// it. The pair below is the other case, and it is the commoner one when
// a document is written from a source that does not state the answer:
// the author knows the field exists, knows they cannot yet say what it
// holds, and writes the marker INSTEAD of the expression.
//
// ⚠ Measured 2026-09-18 while converting decision tables: a transform
// output with `sce:unresolved` and no `expr` was refused as
// `validation/missing-attribute` — "must have an 'expr' attribute". The
// refusal is fatal either way and that is right, since there is nothing
// to generate. What was wrong is the QUESTION. It sent the author to
// write the expression they had just declared they could not write, and
// the placeholder id they DID supply — the one naming what has to be
// settled — appeared nowhere in it.
//
// ⚠⚠ `--strict-unresolved` is not the answer here, and the flag is off
// in both tests below on purpose. It is opt-in, and this document builds
// with it no more than without it; a diagnostic that only told the truth
// when a flag was passed would leave the default path saying the wrong
// thing.

const MARKED_NO_EXPR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="marked_no_expr">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out"
          sce:unresolved="scale-factor"
          sce:unresolved-reason="the source states no multiplier"/>
  </datamodel>
</scxml>
"#;

/// The same document with the marker removed — the author simply left
/// the field blank.
const BARE_NO_EXPR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="bare_no_expr">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out"/>
  </datamodel>
</scxml>
"#;

fn generate_json(doc: &std::path::Path, out: &std::path::Path) -> (Option<i32>, String) {
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            doc.to_str().unwrap(),
            "-l",
            "cpp",
            "-o",
            out.to_str().unwrap(),
            "--error-format=json",
        ])
        .output()
        .expect("spawn sce-codegen");
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

fn first_record(stderr: &str) -> serde_json::Value {
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON diagnostic on stderr:\n{stderr}"));
    serde_json::from_str(line).expect("the diagnostic line is one JSON object")
}

#[test]
fn a_marked_output_with_no_expression_is_refused_as_unresolved() {
    let t = Tmp::new("marked_no_expr");
    let doc = t.write("marked_no_expr.scxml", MARKED_NO_EXPR);
    let (code, stderr) = generate_json(&doc, &t.0);
    assert_ne!(
        code,
        Some(0),
        "an unbuildable document generated:\n{stderr}"
    );

    let record = first_record(&stderr);
    assert_eq!(
        record["code"], "validation/unresolved-placeholder",
        "the author is asked for an attribute rather than for the answer: {record}"
    );
    assert_eq!(
        record["actual"], "scale-factor",
        "the placeholder id does not ride the record: {record}"
    );
    assert!(
        record["message"]
            .as_str()
            .unwrap_or_default()
            .contains("no multiplier"),
        "the author's own reason is dropped: {record}"
    );
    // The marker's own row, not the enclosing `<datamodel>` nor the
    // field's start tag one row up: `actual` is the placeholder id, and
    // the author edits the row that holds it (SCE_ERROR_CONTRACT.md
    // §3.1.1).
    assert_eq!(record["location"]["line"].as_u64(), Some(7), "{record}");
}

#[test]
fn an_unmarked_output_with_no_expression_still_asks_for_the_attribute() {
    // The discriminator for the test above. Without it, a build that
    // answered `unresolved-placeholder` for EVERY missing expression —
    // including one the author merely forgot — would pass that test, and
    // such a build is worse than the one being fixed: it would name a
    // marker the document does not carry.
    let t = Tmp::new("bare_no_expr");
    let doc = t.write("bare_no_expr.scxml", BARE_NO_EXPR);
    let (code, stderr) = generate_json(&doc, &t.0);
    assert_ne!(
        code,
        Some(0),
        "an unbuildable document generated:\n{stderr}"
    );

    let record = first_record(&stderr);
    assert_eq!(
        record["code"], "validation/missing-attribute",
        "a field with no marker is reported as though it carried one: {record}"
    );
    assert_eq!(record["fix"]["attr"], "expr", "{record}");
}
