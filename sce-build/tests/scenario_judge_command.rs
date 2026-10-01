// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen judge-scenarios <set> <trace>` judges what an engine driver
//! observed against what a scenario set expects.
//!
//! The library tests in `src/scenario_judge.rs` call the judge directly, so
//! they would all stay green if the command were unwired, misrouted, or printed
//! something other than what the judge found. What a client consumes is this
//! command's stream, so this file runs it.
//!
//! What it holds:
//!   * a trace of a correct machine is `pass` for every scenario, and the
//!     summary names the engine, the digest of the set and what a pass means
//!   * a check that was observed and did not hold is a `fail` at its step,
//!     and the command still exits 0
//!   * a trace taken against another set, or a set with problems, judges
//!     nothing, and says so
//!   * a file that is not a scenario set, or not an observation trace, is
//!     refused through the door a requirement list uses

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

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
        self.of_kind("judgement")
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no summary record. stdout:\n{}", self.stdout))
    }

    fn verdict(&self, id: &str) -> String {
        self.of_kind("verdict")
            .into_iter()
            .find(|record| record["id"] == id)
            .unwrap_or_else(|| panic!("no verdict for {id}"))["verdict"]
            .as_str()
            .expect("a verdict is text")
            .to_string()
    }
}

fn judge(set: &str, trace: &str) -> Run {
    // `--error-format=json` so a refusal is the record the contract names.
    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("--error-format=json")
        .arg("judge-scenarios")
        .arg(set)
        .arg(trace)
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

fn written(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).expect("write test file");
    path.to_string_lossy().into_owned()
}

fn retry_trace() -> Value {
    let text =
        std::fs::read_to_string(fixtures().join("retry-client.trace.json")).expect("fixture");
    serde_json::from_str(&text).expect("fixture is JSON")
}

#[test]
fn a_trace_of_a_correct_machine_passes_every_scenario() {
    let run = judge(
        &fixture("retry-client.scenarios.json"),
        &fixture("retry-client.trace.json"),
    );
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let summary = run.summary();
    assert_eq!(summary["doc_id"], "retry-client");
    assert_eq!(summary["origin"], "ai-proposed");
    assert_eq!(summary["engine"]["name"], "hand-written");
    assert_eq!(summary["judged"], true);
    assert_eq!(summary["scenarios"], 6);
    assert_eq!(summary["pass"], 6);
    assert_eq!(summary["fail"], 0);
    assert_eq!(summary["not-judged"], 0);
    assert_eq!(summary["problems"], 0);
    assert!(
        summary["means"]
            .as_str()
            .is_some_and(|means| means.contains("on this engine")),
        "a pass is not a claim of correctness, and the stream says so: {summary}"
    );
    assert_eq!(run.of_kind("verdict").len(), 6);
    assert!(run.of_kind("failure").is_empty());
    assert!(run.of_kind("gap").is_empty());
    assert!(run.of_kind("problem").is_empty());
}

#[test]
fn the_summary_names_the_set_by_the_digest_of_its_bytes() {
    let run = judge(
        &fixture("retry-client.scenarios.json"),
        &fixture("retry-client.trace.json"),
    );
    let bytes = std::fs::read(fixtures().join("retry-client.scenarios.json")).expect("fixture");
    let expected = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(run.summary()["set_sha256"], expected.as_str());
}

#[test]
fn a_check_that_does_not_hold_is_a_failure_at_its_step_and_the_command_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut trace = retry_trace();
    // At 600 ms the correct machine reports the timeout; this one retries.
    trace["runs"][2]["observations"][4]["outbound"] = serde_json::json!([{"event": "SendRequest"}]);
    trace["runs"][2]["observations"][4]["finished"] = serde_json::json!(false);
    let broken = written(&dir, "broken.trace.json", &trace.to_string());
    let run = judge(&fixture("retry-client.scenarios.json"), &broken);
    assert_eq!(
        run.code,
        Some(0),
        "findings are not a refusal: {}",
        run.stderr
    );
    assert_eq!(run.verdict("T3-at-most-three"), "fail");
    assert_eq!(run.verdict("T1"), "pass");
    let summary = run.summary();
    assert_eq!(
        (summary["pass"].clone(), summary["fail"].clone()),
        (5.into(), 1.into())
    );
    let failures = run.of_kind("failure");
    assert_eq!(
        failures.len(),
        2,
        "outbound and finished both differ: {failures:?}"
    );
    let outbound = failures
        .iter()
        .find(|f| f["check"] == "outbound")
        .expect("outbound");
    assert_eq!(outbound["scenario"], "T3-at-most-three");
    assert_eq!(outbound["step"], 4);
    assert_eq!(
        outbound["expected"],
        serde_json::json!([{"event": "TimeoutError"}])
    );
    assert_eq!(
        outbound["observed"],
        serde_json::json!([{"event": "SendRequest"}])
    );
}

#[test]
fn what_a_driver_cannot_see_is_a_gap_with_its_reason_and_not_a_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut trace = retry_trace();
    trace["observes"]["outbound"] =
        serde_json::json!({"unavailable": "the route of every output is open"});
    for run in trace["runs"].as_array_mut().expect("runs") {
        for step in run["observations"].as_array_mut().expect("observations") {
            step.as_object_mut()
                .expect("an observation")
                .remove("outbound");
        }
    }
    let blind = written(&dir, "blind.trace.json", &trace.to_string());
    let run = judge(&fixture("retry-client.scenarios.json"), &blind);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let summary = run.summary();
    assert_eq!(summary["pass"], 0);
    assert_eq!(summary["fail"], 0);
    assert_eq!(summary["not-judged"], 6);
    assert!(run.of_kind("failure").is_empty(), "a gap is not a failure");
    let gaps = run.of_kind("gap");
    assert!(!gaps.is_empty());
    assert!(
        gaps.iter().all(|g| g["why"]
            .as_str()
            .is_some_and(|w| w.contains("the route of every output is open"))),
        "{gaps:?}"
    );
}

#[test]
fn a_trace_taken_against_another_set_judges_nothing_and_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut trace = retry_trace();
    trace["scenario_set"] = serde_json::json!({"sha256": "0".repeat(64)});
    let other = written(&dir, "other.trace.json", &trace.to_string());
    let run = judge(&fixture("retry-client.scenarios.json"), &other);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.summary()["judged"], false);
    assert!(run.of_kind("verdict").is_empty());
    let problems = run.of_kind("problem");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0]["code"], "trace-for-another-set");
    assert_eq!(problems[0]["source"], "trace");
}

#[test]
fn a_set_with_problems_is_not_judged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text =
        std::fs::read_to_string(fixtures().join("retry-client.scenarios.json")).expect("fixture");
    let broken = text.replace("\"send\": \"RequestNeeded\"", "\"send\": \"Unheard\"");
    let set = written(&dir, "broken.scenarios.json", &broken);
    let run = judge(&set, &fixture("retry-client.trace.json"));
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.summary()["judged"], false);
    assert!(run.of_kind("verdict").is_empty());
    let problems = run.of_kind("problem");
    assert!(
        problems.iter().all(|p| p["source"] == "set"),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p["code"] == "unknown-input"),
        "{problems:?}"
    );
}

#[test]
fn a_file_that_is_not_what_it_should_be_is_refused_and_prints_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prose = written(&dir, "prose.json", "{ not json");
    let profile = written(
        &dir,
        "profile.json",
        r#"{"record": "sce-authoring-profile", "v": 1}"#,
    );
    let cases: Vec<(&str, String, String)> = vec![
        (
            "a set that is not JSON",
            prose.clone(),
            fixture("retry-client.trace.json"),
        ),
        (
            "a set that is another record",
            profile.clone(),
            fixture("retry-client.trace.json"),
        ),
        (
            "a trace that is not JSON",
            fixture("retry-client.scenarios.json"),
            prose,
        ),
        // a scenario set where the trace should be
        (
            "a trace that is a scenario set",
            fixture("retry-client.scenarios.json"),
            fixture("retry-client.scenarios.json"),
        ),
        (
            "a trace that is another record",
            fixture("retry-client.scenarios.json"),
            profile,
        ),
    ];
    for (what, set, trace) in &cases {
        let run = judge(set, trace);
        // The same door and the same status a requirement list's refusal uses
        // (SCE_ERROR_CONTRACT.md §6).
        assert_eq!(run.code, Some(20), "{what} must be refused: {}", run.stderr);
        assert!(
            run.stdout.is_empty(),
            "{what}: a refused file prints no record: {}",
            run.stdout
        );
        let record: Value = serde_json::from_str(run.stderr.lines().next().expect("a record"))
            .unwrap_or_else(|e| panic!("{what}: stderr is not a record ({e}): {}", run.stderr));
        assert_eq!(
            record["code"], "cli/closure-input-unusable",
            "{what}: {record}"
        );
    }
}
