// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every committed replay scenario keeps to the members its drivers read, and names
//! only what its machine has; and `sce-codegen scenarios` checks a replay scenario
//! as it checks a scenario set (`sce_build::scenario_replay`).
//!
//! # What was wrong
//!
//! Measured 2026-10-21: `sce-codegen scenarios
//! sce-build/tests/fixtures/static_datamodel/scenarios/record_list_parent.json`
//! refused the file as "not a scenario set: its `record` is missing", so a project
//! writing scenarios for its own machines had no way to check one without running a
//! backend's driver, and the members and rules every driver keeps to lived in the
//! Rust driver's crate.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::scenario_replay::{machine_problems, problems, Dialect, MachineFacts};
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The directories whose every `.json` file is a replay scenario a driver replays.
fn scenario_dirs() -> [PathBuf; 2] {
    [
        fixtures().join("scenarios"),
        repo_root().join("tests/host_features/scenarios"),
    ]
}

/// Every committed replay scenario, by its path.
fn committed() -> Vec<(PathBuf, Value)> {
    let mut all = Vec::new();
    for dir in scenario_dirs() {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).expect("a scenario is readable");
            let value: Value = sce_build::strict_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{} is JSON with no key twice: {e}", path.display()));
            all.push((path, value));
        }
    }
    assert!(
        all.len() >= 80,
        "{} scenarios read: a test that read none would pass",
        all.len()
    );
    all
}

/// A member a driver does not know is ignored by most and refused by one, so a
/// misspelt key would pass where nothing reads it. Every committed scenario is held
/// to the vocabulary of its dialect, and one failure names them all.
#[test]
fn every_committed_scenario_keeps_to_the_vocabulary_its_drivers_replay() {
    let mut refused = Vec::new();
    for (path, scenario) in committed() {
        for problem in problems(&scenario) {
            refused.push(format!(
                "{} {} [{}]: {}",
                path.display(),
                problem.path,
                problem.code,
                problem.detail
            ));
        }
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
}

/// The scenarios of the standard host features are of their own dialect, and the
/// machine's are of the machine's: the directory a scenario is in is the drivers
/// that replay it.
#[test]
fn a_scenario_is_of_the_dialect_its_drivers_read() {
    for (path, scenario) in committed() {
        let want = if path.starts_with(repo_root().join("tests/host_features")) {
            Dialect::HostFeatures
        } else {
            Dialect::Machine
        };
        assert_eq!(Dialect::of(&scenario), want, "{}", path.display());
    }
}

/// The machine a scenario names, as a document a parser reads: a fixture beside the
/// scenario directory, or a standard statechart by its library name.
fn machine_document(name: &str) -> String {
    if name.starts_with("sce:std/") {
        format!("{name}.scxml")
    } else {
        fixtures()
            .join(format!("{name}.scxml"))
            .display()
            .to_string()
    }
}

/// Every event a committed scenario sends its machine is one a transition of it
/// matches (or the step says it is dropped), and every state and variable it
/// expects is one the machine's document declares.
#[test]
fn every_committed_scenario_names_only_what_its_machine_has() {
    let mut wrong = Vec::new();
    for (path, scenario) in committed() {
        let name = scenario["machine"].as_str().expect("a machine");
        let document = machine_document(name);
        let model = sce_build::parser::SCXMLParser::new()
            .parse_file(&document)
            .unwrap_or_else(|e| panic!("{document} parses: {e}"));
        let stem = name.rsplit('/').next().unwrap_or(name);
        for problem in machine_problems(&scenario, &MachineFacts::of(&model, stem)) {
            wrong.push(format!(
                "{} {} [{}]: {}",
                path.display(),
                problem.path,
                problem.code,
                problem.detail
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `sce-codegen scenarios <args…>`: its exit status, and each line it wrote as JSON.
fn scenarios(args: &[&str]) -> (bool, Vec<Value>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("scenarios")
        .args(args)
        .output()
        .expect("invoke sce-codegen");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    (
        out.status.success(),
        lines,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_command_reads_a_replay_scenario_and_holds_it_to_its_machine() {
    let scenario = fixtures().join("scenarios/record_list_parent.json");
    let machine = fixtures().join("record_list_parent.scxml");
    let (ok, lines, stderr) = scenarios(&[
        scenario.to_str().expect("a path"),
        "--machine",
        machine.to_str().expect("a path"),
    ]);
    assert!(ok, "the command ran:\n{stderr}");
    let summary = &lines[0];
    assert_eq!(summary["kind"], "replay-scenario", "{lines:?}");
    assert_eq!(summary["dialect"], "machine", "{lines:?}");
    assert_eq!(summary["machine"], "record_list_parent", "{lines:?}");
    assert_eq!(summary["usable"], true, "{lines:?}");
    assert_eq!(
        lines.len(),
        1,
        "a usable scenario has no problem lines: {lines:?}"
    );
}

/// A standard statechart a scenario names by its library name is read from the
/// library without `--machine`.
#[test]
fn a_scenario_of_a_standard_statechart_is_held_to_the_library_document() {
    let scenario = fixtures().join("scenarios/record_list_alone.json");
    let (ok, lines, stderr) = scenarios(&[scenario.to_str().expect("a path")]);
    assert!(ok, "the command ran:\n{stderr}");
    assert_eq!(
        lines[0]["checked_against"], "sce:std/shared/record_list.scxml",
        "{lines:?}"
    );
    assert_eq!(lines[0]["usable"], true, "{lines:?}");
}

/// What is wrong is answered all at once, each problem at its place, and the
/// command still exits 0: the problems are findings.
#[test]
fn the_command_reports_every_problem_of_a_scenario() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("broken.json");
    std::fs::write(
        &path,
        r#"{
  "machine": "static_counter",
  "steps": [
    { "expect": { "state": "nowhere", "hostcalls": [] } },
    { "event": "no.such.event", "expect": { "variables": { "nothing": 1 } } }
  ]
}
"#,
    )
    .expect("write the scenario");
    let machine = fixtures().join("static_counter.scxml");
    let (ok, lines, stderr) = scenarios(&[
        path.to_str().expect("a path"),
        "--machine",
        machine.to_str().expect("a path"),
    ]);
    assert!(ok, "findings are no refusal:\n{stderr}");
    assert_eq!(lines[0]["usable"], false, "{lines:?}");
    let found: Vec<(String, String)> = lines[1..]
        .iter()
        .map(|line| {
            (
                line["code"].as_str().expect("a code").to_string(),
                line["path"].as_str().expect("a path").to_string(),
            )
        })
        .collect();
    for (code, at) in [
        ("unknown-member", "/steps/0/expect/hostcalls"),
        ("unknown-state", "/steps/0/expect/state"),
        ("event-unmatched", "/steps/1/event"),
        ("unknown-variable", "/steps/1/expect/variables/nothing"),
    ] {
        assert!(
            found.contains(&(code.to_string(), at.to_string())),
            "expected {code} at {at} among {found:?}"
        );
    }
}

/// A file that is neither a scenario set nor a replay scenario is refused, as it was.
#[test]
fn a_file_that_is_no_scenario_is_still_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("other.json");
    std::fs::write(&path, r#"{"name": "not a scenario"}"#).expect("write the file");
    let (ok, _, stderr) = scenarios(&[path.to_str().expect("a path")]);
    assert!(!ok, "refused:\n{stderr}");
    assert!(
        stderr.contains("cli/closure-input-unusable"),
        "refused as unusable:\n{stderr}"
    );
}
