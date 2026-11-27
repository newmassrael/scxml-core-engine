// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The run trace form (docs/adr/0115, D4 and D8), held from both sides as the
//! inbox history's is: the schema that documents the form accepts exactly the
//! runs the reader accepts, every recorded run this repository judges is one,
//! and a run written by the reader's own writer reads back the same.

use std::path::{Path, PathBuf};

use sce_build::run_trace::{Fired, Microstep, RunTrace, Step, KINDS, TRACE_FORMAT_VERSION};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn validator() -> jsonschema::JSONSchema {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("tests/forge/conformance/run_trace.schema.json"))
            .expect("read the schema"),
    )
    .expect("the schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the schema compiles")
}

/// Every recorded run under `docs/formal/traces`.
fn recorded() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(repo_root().join("docs/formal/traces"))
        .expect("the recorded runs exist")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_recorded_run_is_a_run_trace() {
    let files = recorded();
    assert!(
        files.len() >= 3,
        "one recorded run per hand-written instance of AppendixD; {} here",
        files.len()
    );
    let validator = validator();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read the run");
        let run = RunTrace::from_json(&text)
            .unwrap_or_else(|e| panic!("{} is no run trace: {e}", file.display()));
        assert!(!run.steps.is_empty());
        let document: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert!(
            validator.validate(&document).is_ok(),
            "{} is refused by the schema",
            file.display()
        );
    }
}

const START: &str = r#"{"kind": "start", "entered": ["s0"], "configuration": ["s0"]}"#;

fn run(steps: &str) -> String {
    format!(r#"{{"version": 1, "lane": "rust", "steps": [{steps}]}}"#)
}

/// Documents, each with a name that says what it is: those named `run_` are
/// runs, the others are not.
fn corpus() -> Vec<(&'static str, String)> {
    let microstep = r#""fired": [{"source": "s0", "index": 0}], "exited": ["s0"], "entered": ["s1"], "configuration": ["s1"]"#;
    vec![
        ("run_start_alone", run(START)),
        (
            "run_every_kind",
            run(&format!(
                r#"{START}, {{"kind": "eventless", {microstep}}}, {{"kind": "internal", "event": "a.b", {microstep}}}, {{"kind": "external", "event": "go", {microstep}}}, {{"kind": "stop", "exited": ["s1"], "outcome": "s1"}}"#
            )),
        ),
        (
            "run_an_event_that_selects_nothing",
            run(&format!(
                r#"{START}, {{"kind": "internal", "event": "e", "fired": [], "exited": [], "entered": [], "configuration": ["s0"]}}"#
            )),
        ),
        (
            "run_a_stop_that_ends_in_no_final_state",
            run(&format!(
                r#"{START}, {{"kind": "stop", "exited": ["s0"], "outcome": null}}"#
            )),
        ),
        ("not_json", "not json".to_string()),
        (
            "another_version",
            format!(r#"{{"version": 2, "lane": "rust", "steps": [{START}]}}"#),
        ),
        (
            "no_lane",
            format!(r#"{{"version": 1, "steps": [{START}]}}"#),
        ),
        (
            "an_empty_lane",
            format!(r#"{{"version": 1, "lane": "", "steps": [{START}]}}"#),
        ),
        ("no_step", run("")),
        (
            "an_unknown_field",
            format!(r#"{{"version": 1, "lane": "rust", "steps": [{START}], "note": "x"}}"#),
        ),
        ("an_unknown_kind", run(r#"{"kind": "pause"}"#)),
        (
            "a_start_without_its_configuration",
            run(r#"{"kind": "start", "entered": ["s0"]}"#),
        ),
        (
            "a_start_with_an_event",
            run(r#"{"kind": "start", "event": "e", "entered": ["s0"], "configuration": ["s0"]}"#),
        ),
        (
            "an_eventless_step_with_an_event",
            run(&format!(
                r#"{START}, {{"kind": "eventless", "event": "e", {microstep}}}"#
            )),
        ),
        (
            "an_internal_step_without_its_event",
            run(&format!(r#"{START}, {{"kind": "internal", {microstep}}}"#)),
        ),
        (
            "an_external_step_without_what_it_exited",
            run(&format!(
                r#"{START}, {{"kind": "external", "event": "e", "fired": [], "entered": [], "configuration": []}}"#
            )),
        ),
        (
            "a_stop_without_its_outcome",
            run(&format!(r#"{START}, {{"kind": "stop", "exited": ["s0"]}}"#)),
        ),
        (
            "a_stop_with_a_configuration",
            run(&format!(
                r#"{START}, {{"kind": "stop", "exited": ["s0"], "outcome": null, "configuration": []}}"#
            )),
        ),
        (
            "an_event_name_with_an_empty_token",
            run(&format!(
                r#"{START}, {{"kind": "internal", "event": "a..b", {microstep}}}"#
            )),
        ),
        (
            "a_configuration_that_names_a_state_twice",
            run(r#"{"kind": "start", "entered": ["s0"], "configuration": ["s0", "s0"]}"#),
        ),
        (
            "a_state_of_an_empty_id",
            run(r#"{"kind": "start", "entered": [""], "configuration": []}"#),
        ),
        (
            "a_negative_index",
            run(&format!(
                r#"{START}, {{"kind": "eventless", "fired": [{{"source": "s0", "index": -1}}], "exited": [], "entered": [], "configuration": []}}"#
            )),
        ),
        (
            "a_row_with_an_unknown_field",
            run(&format!(
                r#"{START}, {{"kind": "eventless", "fired": [{{"source": "s0", "index": 0, "event": "e"}}], "exited": [], "entered": [], "configuration": []}}"#
            )),
        ),
        (
            "an_outcome_of_an_empty_id",
            run(&format!(
                r#"{START}, {{"kind": "stop", "exited": [], "outcome": ""}}"#
            )),
        ),
    ]
}

/// The schema is as strict as the reader: a run the reader takes validates
/// against it, and one the reader refuses does not.
#[test]
fn the_schema_accepts_what_the_reader_accepts() {
    let validator = validator();
    for (name, text) in corpus() {
        let reader_takes_it = RunTrace::from_json(&text).is_ok();
        assert_eq!(
            reader_takes_it,
            name.starts_with("run_"),
            "{name}: the reader says {reader_takes_it}"
        );
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&text) else {
            assert!(
                !reader_takes_it,
                "{name}: the reader took a text that is no JSON"
            );
            continue;
        };
        let valid = validator.validate(&document).is_ok();
        assert_eq!(
            valid, reader_takes_it,
            "{name}: the schema says {valid}, the reader says {reader_takes_it}"
        );
    }
}

/// What the writer writes, the reader reads back, every kind of step and an
/// index past 2^31 among it.
#[test]
fn a_written_run_reads_back_the_same() {
    let names = |listed: &[&str]| -> Vec<String> { listed.iter().map(|n| n.to_string()).collect() };
    let microstep = Microstep {
        fired: vec![Fired {
            source: "s0".to_string(),
            index: u32::MAX,
        }],
        exited: names(&["s01", "s0"]),
        entered: names(&["s1"]),
        configuration: names(&["s1"]),
    };
    let run = RunTrace {
        lane: "c11".to_string(),
        steps: vec![
            Step::Start {
                entered: names(&["s0", "s01"]),
                configuration: names(&["s0", "s01"]),
            },
            Step::Eventless(microstep.clone()),
            Step::Internal {
                event: "done.state.s0".to_string(),
                microstep: microstep.clone(),
            },
            Step::External {
                event: "go".to_string(),
                microstep,
            },
            Step::Stop {
                exited: names(&["s1"]),
                outcome: None,
            },
        ],
    };
    let written = run.to_json();
    let read = RunTrace::from_json(&written).expect("the writer's output reads");
    assert_eq!(read, run);
    assert_eq!(
        read.to_json(),
        written,
        "writing what was read gives the same text"
    );
    let kinds: Vec<&str> = read.steps.iter().map(Step::kind).collect();
    assert_eq!(kinds, KINDS, "every kind of step is written");
    let document: serde_json::Value = serde_json::from_str(&written).expect("JSON");
    assert_eq!(document["version"], TRACE_FORMAT_VERSION);
    assert!(validator().validate(&document).is_ok());
}
