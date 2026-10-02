// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen requirements <document> --manifest <list> --scenarios <set>
//! --trace <trace>` holds a requirement list against examples that were played.
//!
//! A requirement met by something NOT happening has no node to point at, so the
//! annotation can only call it `needs-scenario`. The library tests in
//! `src/scenario_closure.rs` hold the rule; this file runs the command, since
//! they would all stay green if the flags were unwired, the set were read as a
//! verdict file, or the summary were left out of the stream.
//!
//! What it holds:
//!   * a `shall_not` whose every scenario passed is `scenario-passed`, and one
//!     with a failed scenario `scenario-failed`; the rows carry their scenarios'
//!     verdicts
//!   * a `scenario-evidence` record comes before the first row, naming the
//!     engine and the set's digest and `origin`
//!   * without the flags the stream is what it always was: no evidence record,
//!     and no row moved
//!   * examples about another specification move nothing and say why
//!   * the flags come as a pair, and a set or trace that does not load ends the
//!     run through the door a requirement list uses

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

const DOCUMENT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a"/>
</scxml>"#;

const MANIFEST: &str = r#"{
  "doc_id": "retry", "rev": "1",
  "extraction": {"ids": "native", "trace": "none",
                 "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
  "sections": [{"id": "1", "title": "Retry"}],
  "requirements": [
    {"id": "R1", "section": "1", "modality": "shall_not", "at": {"page": 1}},
    {"id": "R2", "section": "1", "modality": "shall_not", "at": {"page": 1}},
    {"id": "R3", "section": "1", "at": {"page": 1}},
    {"id": "R4", "section": "1", "modality": "shall_not", "at": {"page": 1}}
  ]
}"#;

fn set_text(doc_id: &str) -> String {
    format!(
        r#"{{
      "record": "sce-scenario-set", "v": 1,
      "specification": {{"doc_id": "{doc_id}", "rev": "1"}},
      "origin": "ai-proposed",
      "interface": {{"inputs": [], "outputs": []}},
      "scenarios": [
        {{"id": "S-holds", "quote": "q1", "requirements": ["R1"],
          "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
        {{"id": "S-breaks", "quote": "q2", "requirements": ["R2", "R3"],
          "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}},
        {{"id": "S-open", "quote": "q3", "requirements": ["R4"], "status": "awaiting-decision",
          "decision": {{"question": "Who is the caller?"}},
          "steps": [{{"advance_ms": 5, "expect": {{"outbound": []}}}}]}}
      ]
    }}"#
    )
}

const TRACE: &str = r#"{
  "record": "sce-observation-trace", "v": 1,
  "engine": {"name": "test engine", "detail": "hand-written"},
  "observes": {"outbound": true, "finished": true, "configuration": true, "data": true},
  "runs": [
    {"scenario": "S-holds",
     "observations": [{"outbound": [], "finished": false, "configuration": [], "data": {}}]},
    {"scenario": "S-breaks",
     "observations": [{"outbound": [{"event": "Late"}], "finished": false,
                       "configuration": [], "data": {}}]}
  ]
}"#;

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    fn records(&self) -> Vec<Value> {
        self.stdout
            .lines()
            .map(|line| serde_json::from_str(line).expect("each line is a JSON record"))
            .collect()
    }

    fn row(&self, id: &str) -> Value {
        self.records()
            .into_iter()
            .find(|r| r["kind"] == "requirement" && r["id"] == id)
            .unwrap_or_else(|| panic!("no row for {id}. stdout:\n{}", self.stdout))
    }
}

struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn new() -> Self {
        Files {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    fn write(&self, name: &str, text: &str) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, text).expect("write test file");
        path.to_string_lossy().into_owned()
    }
}

fn requirements(args: &[&str]) -> Run {
    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("--error-format=json")
        .arg("requirements")
        .args(args)
        .output()
        .expect("sce-codegen runs");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout).expect("utf-8 stdout"),
        stderr: String::from_utf8(output.stderr).expect("utf-8 stderr"),
    }
}

#[test]
fn played_examples_move_the_requirements_they_name() {
    let files = Files::new();
    let document = files.write("design.scxml", DOCUMENT);
    let manifest = files.write("list.json", MANIFEST);
    let set = files.write("set.json", &set_text("retry"));
    let trace = files.write("trace.json", TRACE);
    let run = requirements(&[
        &document,
        "--manifest",
        &manifest,
        "--scenarios",
        &set,
        "--trace",
        &trace,
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);

    let held = run.row("R1");
    assert_eq!(held["outcome"], "scenario-passed", "{held}");
    assert_eq!(held["scenarios"][0]["scenario"], "S-holds");
    assert_eq!(held["scenarios"][0]["verdict"], "pass");
    // A requirement the annotation called `missing` (a `shall`, nothing cites
    // it) and one it called `needs-scenario` are both failed by the example
    // that names them.
    assert_eq!(run.row("R2")["outcome"], "scenario-failed");
    assert_eq!(run.row("R3")["outcome"], "scenario-failed");
    // An example awaiting a decision closes nothing, and is carried on the row.
    let open = run.row("R4");
    assert_eq!(open["outcome"], "needs-scenario", "{open}");
    assert_eq!(open["scenarios"][0]["verdict"], "awaiting-decision");
}

#[test]
fn the_evidence_record_comes_before_the_first_row_and_names_what_it_is_about() {
    let files = Files::new();
    let run = requirements(&[
        &files.write("design.scxml", DOCUMENT),
        "--manifest",
        &files.write("list.json", MANIFEST),
        "--scenarios",
        &files.write("set.json", &set_text("retry")),
        "--trace",
        &files.write("trace.json", TRACE),
    ]);
    let records = run.records();
    assert_eq!(records[0]["kind"], "extraction");
    let evidence = &records[1];
    assert_eq!(evidence["kind"], "scenario-evidence", "{evidence}");
    assert_eq!(evidence["used"], true);
    assert_eq!(evidence["origin"], "ai-proposed");
    assert_eq!(evidence["engine"]["name"], "test engine");
    assert_eq!(evidence["engine"]["detail"], "hand-written");
    assert_eq!(evidence["doc_id"], "retry");
    assert!(
        evidence["set_sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64),
        "{evidence}"
    );
    assert_eq!(records[2]["kind"], "requirement");
}

#[test]
fn without_the_examples_the_stream_is_what_it_always_was() {
    let files = Files::new();
    let run = requirements(&[
        &files.write("design.scxml", DOCUMENT),
        "--manifest",
        &files.write("list.json", MANIFEST),
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.records()
            .iter()
            .all(|r| r["kind"] != "scenario-evidence"),
        "{}",
        run.stdout
    );
    assert_eq!(run.row("R1")["outcome"], "needs-scenario");
    assert_eq!(run.row("R3")["outcome"], "missing");
    assert!(run.row("R1").get("scenarios").is_none());
}

#[test]
fn examples_about_another_specification_move_nothing_and_say_so() {
    let files = Files::new();
    let run = requirements(&[
        &files.write("design.scxml", DOCUMENT),
        "--manifest",
        &files.write("list.json", MANIFEST),
        "--scenarios",
        &files.write("set.json", &set_text("another-spec")),
        "--trace",
        &files.write("trace.json", TRACE),
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let evidence = run.records()[1].clone();
    assert_eq!(evidence["kind"], "scenario-evidence");
    assert_eq!(evidence["used"], false, "{evidence}");
    assert!(
        evidence["why_not_used"]
            .as_str()
            .is_some_and(|why| why.contains("another-spec@1") && why.contains("retry@1")),
        "{evidence}"
    );
    assert_eq!(run.row("R1")["outcome"], "needs-scenario");
}

#[test]
fn the_flags_come_as_a_pair_and_with_a_list() {
    let files = Files::new();
    let document = files.write("design.scxml", DOCUMENT);
    let manifest = files.write("list.json", MANIFEST);
    let set = files.write("set.json", &set_text("retry"));
    let trace = files.write("trace.json", TRACE);
    for (what, args) in [
        (
            "a set with no trace",
            vec![&document, "--manifest", &manifest, "--scenarios", &set],
        ),
        (
            "a trace with no set",
            vec![&document, "--manifest", &manifest, "--trace", &trace],
        ),
        (
            "examples with no requirement list",
            vec![&document, "--scenarios", &set, "--trace", &trace],
        ),
    ] {
        let args: Vec<&str> = args.iter().map(|a| a.as_ref()).collect();
        let run = requirements(&args);
        assert_ne!(run.code, Some(0), "{what} must be refused");
        assert!(run.stdout.is_empty(), "{what}: {}", run.stdout);
    }
}

#[test]
fn a_set_or_a_trace_that_does_not_load_ends_the_run_naming_the_file() {
    let files = Files::new();
    let document = files.write("design.scxml", DOCUMENT);
    let manifest = files.write("list.json", MANIFEST);
    let set = files.write("set.json", &set_text("retry"));
    let trace = files.write("trace.json", TRACE);
    let broken = files.write("broken.json", "{ not json");
    for (what, set_file, trace_file) in [("set", &broken, &trace), ("trace", &set, &broken)] {
        let run = requirements(&[
            &document,
            "--manifest",
            &manifest,
            "--scenarios",
            set_file,
            "--trace",
            trace_file,
        ]);
        assert_eq!(run.code, Some(20), "{what}: {}", run.stderr);
        assert!(
            run.stdout.is_empty(),
            "{what} prints no record: {}",
            run.stdout
        );
        let record: Value = serde_json::from_str(run.stderr.lines().next().expect("a record"))
            .unwrap_or_else(|e| panic!("{what}: stderr is not a record ({e}): {}", run.stderr));
        assert_eq!(record["code"], "cli/closure-input-unusable", "{record}");
        assert!(
            record["message"]
                .as_str()
                .is_some_and(|m| m.contains("broken.json")),
            "{what}: the refusal names the file: {record}"
        );
    }
}

#[test]
fn several_documents_read_as_one_design_are_held_against_the_examples_too() {
    let files = Files::new();
    let run = requirements(&[
        &files.write("design.scxml", DOCUMENT),
        &files.write("other.scxml", DOCUMENT),
        "--manifest",
        &files.write("list.json", MANIFEST),
        "--scenarios",
        &files.write("set.json", &set_text("retry")),
        "--trace",
        &files.write("trace.json", TRACE),
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.row("R1")["outcome"], "scenario-passed");
    assert_eq!(run.records()[1]["kind"], "scenario-evidence");
}
