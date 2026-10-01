// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen scenarios <file> [--specification <path>]` reads a scenario set
//! and reports it.
//!
//! The library tests in `src/scenario_set.rs` call the reader directly, so they
//! would all stay green if the command were unwired, misrouted, or printed
//! something other than what the reader found. What an author's tool consumes
//! is this command's stream, so this file runs it.
//!
//! What it holds:
//!   * a usable set is answered with a summary, a record per scenario and no
//!     problem, and exits 0
//!   * problems are FINDINGS: exit 0, `usable: false`, a record per problem
//!   * a quote is checked only when the specification is given, and the
//!     scenario whose quote is missing is the one that says so
//!   * a file that is not a scenario set is refused through the door a
//!     requirement list uses, with nothing on stdout

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scenario_sets")
}

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

    fn of_kind(&self, kind: &str) -> Vec<Value> {
        self.records()
            .into_iter()
            .filter(|record| record["kind"] == kind)
            .collect()
    }

    fn summary(&self) -> Value {
        self.of_kind("scenario-set")
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no summary record. stdout:\n{}", self.stdout))
    }
}

fn scenarios(args: &[&str]) -> Run {
    // `--error-format=json` so a refusal is the record the contract names, not
    // prose that can be reworded. A run that refuses nothing is unaffected.
    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("--error-format=json")
        .arg("scenarios")
        .args(args)
        .output()
        .expect("sce-codegen runs");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout).expect("utf-8 stdout"),
        stderr: String::from_utf8(output.stderr).expect("utf-8 stderr"),
    }
}

fn fixture(name: &str) -> String {
    fixtures().join(name).to_string_lossy().into_owned()
}

/// A scenario set written to a file of its own, for the cases that need one
/// the fixtures are not.
fn written(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).expect("write test file");
    path.to_string_lossy().into_owned()
}

const SMALL: &str = r#"{
  "record": "sce-scenario-set", "v": 1,
  "specification": {"doc_id": "d", "rev": "1"},
  "origin": "owner-written",
  "interface": {"inputs": [{"name": "go"}], "outputs": [{"name": "done"}]},
  "scenarios": [
    {"id": "S1", "quote": "The machine goes.",
     "steps": [{"send": "go", "expect": {"outbound": [{"event": "done"}]}}]},
    {"id": "S2", "quote": "It never stops.",
     "steps": [{"advance_ms": 5, "expect": {"outbound": []}}]}
  ]
}"#;

#[test]
fn a_usable_set_is_answered_with_a_summary_and_a_record_per_scenario() {
    let run = scenarios(&[
        &fixture("retry-client.scenarios.json"),
        "--specification",
        &fixture("retry-client.spec.txt"),
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let summary = run.summary();
    assert_eq!(summary["doc_id"], "retry-client");
    assert_eq!(summary["origin"], "ai-proposed");
    assert_eq!(summary["scenarios"], 6);
    assert_eq!(summary["runnable"], 6);
    assert_eq!(summary["quotes_checked"], true);
    assert_eq!(summary["problems"], 0);
    assert_eq!(summary["usable"], true);
    let records = run.of_kind("scenario");
    assert_eq!(records.len(), 6);
    assert!(
        records.iter().all(|r| r["quote_found"] == true),
        "{records:?}"
    );
    assert!(run.of_kind("problem").is_empty());
    let boundary = records
        .iter()
        .find(|r| r["id"] == "T2-boundary")
        .expect("T2");
    assert_eq!(boundary["steps"], 3);
    assert_eq!(boundary["expectations"], 3);
}

#[test]
fn every_set_the_product_ships_is_usable_and_says_what_it_holds() {
    // (file, scenarios, runnable, awaiting, blocked)
    for (name, total, runnable, awaiting, blocked) in [
        ("door-with-auto-close", 7, 5, 2, 0),
        ("connection-keeper", 8, 8, 0, 0),
        ("vending-controller", 13, 6, 1, 6),
        ("retry-client", 6, 6, 0, 0),
    ] {
        let run = scenarios(&[
            &fixture(&format!("{name}.scenarios.json")),
            "--specification",
            &fixture(&format!("{name}.spec.txt")),
        ]);
        assert_eq!(run.code, Some(0), "{name}: {}", run.stderr);
        let summary = run.summary();
        assert_eq!(summary["usable"], true, "{name}: {}", run.stdout);
        assert_eq!(
            (
                &summary["scenarios"],
                &summary["runnable"],
                &summary["awaiting-decision"],
                &summary["blocked"]
            ),
            (
                &Value::from(total),
                &Value::from(runnable),
                &Value::from(awaiting),
                &Value::from(blocked)
            ),
            "{name}"
        );
        assert_eq!(run.of_kind("scenario").len(), total, "{name}");
    }
}

#[test]
fn a_quote_is_checked_only_when_the_specification_is_given() {
    let dir = tempfile::tempdir().expect("tempdir");
    let set = written(&dir, "small.json", SMALL);
    let without = scenarios(&[&set]);
    assert_eq!(without.code, Some(0), "stderr: {}", without.stderr);
    assert_eq!(without.summary()["quotes_checked"], false);
    assert_eq!(without.summary()["usable"], true);
    assert!(
        without
            .of_kind("scenario")
            .iter()
            .all(|r| r["quote_found"].is_null()),
        "no quote was checked, so none is said to be found or missing"
    );
}

#[test]
fn the_scenario_whose_quote_is_missing_is_the_one_that_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let set = written(&dir, "small.json", SMALL);
    let spec = written(&dir, "spec.txt", "The machine goes. It stops.");
    let run = scenarios(&[&set, "--specification", &spec]);
    assert_eq!(
        run.code,
        Some(0),
        "problems are findings, not a refusal: {}",
        run.stderr
    );
    let summary = run.summary();
    assert_eq!(summary["usable"], false);
    assert_eq!(summary["problems"], 1);
    let by_id = |id: &str| {
        run.of_kind("scenario")
            .into_iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("no record for {id}"))
    };
    assert_eq!(by_id("S1")["quote_found"], true);
    assert_eq!(by_id("S2")["quote_found"], false);
    let problems = run.of_kind("problem");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0]["code"], "quote-not-found");
    assert_eq!(problems[0]["scenario"], "S2");
    assert_eq!(problems[0]["path"], "scenarios[1].quote");
}

#[test]
fn every_problem_is_reported_and_the_command_still_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let broken = SMALL
        .replace("\"send\": \"go\"", "\"send\": \"stop\"")
        .replace("\"event\": \"done\"", "\"event\": \"vanished\"");
    let set = written(&dir, "broken.json", &broken);
    let run = scenarios(&[&set]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.summary()["usable"], false);
    let codes: Vec<String> = run
        .of_kind("problem")
        .iter()
        .map(|p| p["code"].as_str().expect("a code").to_string())
        .collect();
    assert!(codes.contains(&"unknown-input".to_string()), "{codes:?}");
    assert!(codes.contains(&"unknown-output".to_string()), "{codes:?}");
}

#[test]
fn a_file_that_is_not_a_scenario_set_is_refused_and_prints_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in [
        ("prose.json", "{ not json"),
        (
            "profile.json",
            r#"{"record": "sce-authoring-profile", "v": 1}"#,
        ),
        ("future.json", r#"{"record": "sce-scenario-set", "v": 9}"#),
        (
            "extra.json",
            &SMALL.replace("\"origin\"", "\"surprise\": 1, \"origin\""),
        ),
    ] {
        let path = written(&dir, name, text);
        let run = scenarios(&[&path]);
        // The same door and the same status a requirement list's refusal uses
        // (SCE_ERROR_CONTRACT.md §6), so one probe on a loader is not the
        // only evidence about the next.
        assert_eq!(run.code, Some(20), "{name} must be refused: {}", run.stderr);
        assert!(
            run.stdout.is_empty(),
            "{name}: a refused file prints no record: {}",
            run.stdout
        );
        let record: Value = serde_json::from_str(run.stderr.lines().next().expect("a record"))
            .unwrap_or_else(|e| panic!("{name}: stderr is not a record ({e}): {}", run.stderr));
        assert_eq!(
            record["code"], "cli/closure-input-unusable",
            "{name}: {record}"
        );
        assert!(
            record["message"].as_str().is_some_and(|m| m.contains(name)),
            "{name}: the refusal names the file: {record}"
        );
    }
}

#[test]
fn a_specification_that_cannot_be_read_ends_the_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let set = written(&dir, "small.json", SMALL);
    let run = scenarios(&[&set, "--specification", "/no/such/specification.txt"]);
    assert_ne!(run.code, Some(0));
    assert!(run.stdout.is_empty(), "{}", run.stdout);
}
