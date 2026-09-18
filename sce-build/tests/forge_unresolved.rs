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
